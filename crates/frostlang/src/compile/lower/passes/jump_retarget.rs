//! Jump retargeting: a jump that lands on another jump whose outcome is already
//! known on arrival goes straight on to where that one leads.
//!
//! A jump lands on the first instruction after its label. Each round takes one
//! step for every jump it can:
//! - A jump landing on an unconditional `Jump` takes that jump's target.
//! - A peek jump leaves the value it tested, so one landing on another peek
//!   jump knows that jump's outcome: of the same sense, it takes its target; of
//!   the opposite sense, it goes on just past it.
//! - A peek jump directly followed by the `Pop` that discards its value,
//!   landing on a consuming test, becomes one consuming jump of its own sense,
//!   targeted as for peek jumps above: where it jumps, the test it skips would
//!   have consumed the value; where it falls through, the `Pop` would have. A
//!   label between the peek jump and the `Pop` means some jump lands on the
//!   `Pop`, so they stay.
//!
//! A step can open another, so rounds repeat until one changes nothing. Every
//! jump goes forward, so each step moves a target strictly later and the rounds
//! stop; `MAX_ROUNDS` only bounds the work.

use std::collections::HashMap;

use crate::bytecode::Bytecode;
use crate::compile::lower::{Ir, JumpType, Label, LoweredFunction, Op};

/// More rounds than real code needs. Stopping early leaves correct code, only
/// with longer paths.
const MAX_ROUNDS: usize = 32;

pub(super) fn retarget_jumps(mut function: LoweredFunction) -> LoweredFunction {
    for _ in 0..MAX_ROUNDS {
        let mut changed = false;
        // Fusing removes a `Pop` that a retarget in the same round could land a
        // jump on, so the two take turns.
        for round in [retarget_round, fuse_round] {
            if let Some(rebuilt) = round(&function) {
                function.code = rebuilt.code;
                function.num_labels = rebuilt.num_labels;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    function
}

/// Retarget, by one step, each jump landing on a jump whose outcome it already
/// knows. `None` if there is none.
fn retarget_round(function: &LoweredFunction) -> Option<RebuiltCode> {
    let mut round = Round::new(function);
    for (index, ir) in function.code.iter().enumerate() {
        let Op::Jump { kind, label } = ir.op else {
            continue;
        };
        let Some(landing) = round.landing(label) else {
            continue;
        };
        let Op::Jump {
            kind: landed_kind,
            label: landed_label,
        } = function.code[landing].op
        else {
            continue;
        };
        let target = match (kind, landed_kind) {
            (_, JumpType::Unconditional)
            | (JumpType::PeekIfTrue, JumpType::PeekIfTrue)
            | (JumpType::PeekIfFalse, JumpType::PeekIfFalse) => landed_label,
            (JumpType::PeekIfTrue, JumpType::PeekIfFalse)
            | (JumpType::PeekIfFalse, JumpType::PeekIfTrue) => round.label_after(landing),
            _ => continue,
        };
        round.replace(
            index,
            Op::Jump {
                kind,
                label: target,
            },
        );
    }
    round.finish()
}

/// Fuse each peek jump and the `Pop` directly after it that land on a consuming
/// test into one consuming jump. `None` if there is none.
fn fuse_round(function: &LoweredFunction) -> Option<RebuiltCode> {
    let mut round = Round::new(function);
    for (index, pair) in function.code.windows(2).enumerate() {
        let (Op::Jump { kind, label }, Op::Ready(Bytecode::Pop)) = (&pair[0].op, &pair[1].op)
        else {
            continue;
        };
        let fused_kind = match kind {
            JumpType::PeekIfTrue => JumpType::IfTrue,
            JumpType::PeekIfFalse => JumpType::IfFalse,
            _ => continue,
        };
        let Some(landing) = round.landing(*label) else {
            continue;
        };
        let Op::Jump {
            kind: landed_kind @ (JumpType::IfTrue | JumpType::IfFalse),
            label: landed_label,
        } = function.code[landing].op
        else {
            continue;
        };
        // The fused jump tests the value the test would: of the same sense, it
        // goes where the test goes, otherwise just past it.
        let target = if fused_kind == landed_kind {
            landed_label
        } else {
            round.label_after(landing)
        };
        round.replace(
            index,
            Op::Jump {
                kind: fused_kind,
                label: target,
            },
        );
        round.remove(index + 1);
    }
    round.finish()
}

/// A function's code as a round rebuilt it, and how many labels it now mints.
struct RebuiltCode {
    code: Vec<Ir>,
    num_labels: usize,
}

/// One round's edits of a function's code, decided against the code as the
/// round found it, then applied together.
struct Round<'a> {
    code: &'a [Ir],
    // The index in `code` of each placed label.
    label_indices: Vec<Option<usize>>,
    num_labels: usize,
    // Each new label, by the index of the instruction it goes before.
    new_labels: HashMap<usize, Label>,
    edits: HashMap<usize, Edit>,
}

/// What a round does to one instruction.
enum Edit {
    Replace(Op),
    Remove,
}

impl<'a> Round<'a> {
    fn new(function: &'a LoweredFunction) -> Self {
        let mut label_indices = vec![None; function.num_labels];
        for (index, ir) in function.code.iter().enumerate() {
            if let Op::Label(label) = ir.op {
                label_indices[label.0] = Some(index);
            }
        }
        Self {
            code: &function.code,
            label_indices,
            num_labels: function.num_labels,
            new_labels: HashMap::new(),
            edits: HashMap::new(),
        }
    }

    /// The index of the instruction a jump to `label` lands on, or `None` if
    /// the label is at the end of the code.
    fn landing(&self, label: Label) -> Option<usize> {
        let placed = self.label_indices[label.0].expect("a jump's label is placed");
        (placed..self.code.len()).find(|&index| !matches!(self.code[index].op, Op::Label(_)))
    }

    /// A label for the instruction just past the forward jump at `jump`: one
    /// already there, or a new one. The jump's own label comes later, so that
    /// instruction exists.
    fn label_after(&mut self, jump: usize) -> Label {
        if let Op::Label(label) = self.code[jump + 1].op {
            return label;
        }
        *self.new_labels.entry(jump + 1).or_insert_with(|| {
            let label = Label(self.num_labels);
            self.num_labels += 1;
            label
        })
    }

    fn replace(&mut self, index: usize, op: Op) {
        self.edits.insert(index, Edit::Replace(op));
    }

    fn remove(&mut self, index: usize) {
        self.edits.insert(index, Edit::Remove);
    }

    /// The code with the round's edits applied, or `None` if it made none.
    fn finish(self) -> Option<RebuiltCode> {
        if self.edits.is_empty() {
            return None;
        }
        let mut code = Vec::with_capacity(self.code.len() + self.new_labels.len());
        for (index, ir) in self.code.iter().enumerate() {
            // A new label takes the span of the instruction it goes before.
            if let Some(&label) = self.new_labels.get(&index) {
                code.push(Op::Label(label).at(ir.span));
            }
            match self.edits.get(&index) {
                Some(Edit::Replace(op)) => code.push(op.clone().at(ir.span)),
                Some(Edit::Remove) => {}
                None => code.push(ir.clone()),
            }
        }
        Some(RebuiltCode {
            code,
            num_labels: self.num_labels,
        })
    }
}
