//! [`Disassembly`]: a readable listing of a [`CompiledFunction`] tree.

use std::fmt::{self, Display, Formatter};

use crate::bytecode::{Bytecode, CompiledFunction, GLOBAL_NAMES};
use crate::core::util::identifier::is_identifier_like_and_not_keyword;
use crate::{Arity, MapKey, Value};

/// The widest a constant shown beside an instruction may be before it is cut
/// short. The constant pool itself shows each in full.
const INLINE_CONSTANT_WIDTH: usize = 40;

impl CompiledFunction {
    /// A readable listing of this function, then of each function nested in
    /// it, depth first. Print it with `{}`.
    ///
    /// Each listing shows the function's arity, its slots, constants, Map keys,
    /// and nested functions, then its code. Jumps name labeled targets, and
    /// each instruction that refers to a slot, constant, key, global, or
    /// nested function is annotated with what it refers to.
    ///
    /// The listing is meant for people, and its format may change. Malformed
    /// bytecode lists without panicking: a reference to nothing shows as `?`.
    pub fn disassemble(&self) -> Disassembly<'_> {
        Disassembly {
            function: self,
            color: false,
        }
    }
}

/// A listing of a [`CompiledFunction`] tree, from
/// [`CompiledFunction::disassemble`].
#[derive(Clone, Copy, Debug)]
pub struct Disassembly<'a> {
    function: &'a CompiledFunction,
    color: bool,
}

impl Disassembly<'_> {
    /// Set whether the listing is colored with ANSI escape codes, for a
    /// terminal. It is not, by default.
    pub fn with_color(mut self, color: bool) -> Self {
        self.color = color;
        self
    }
}

impl Display for Disassembly<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let paint = Paint(self.color);
        list(f, self.function, None, paint)?;
        list_children(f, self.function, "", paint)
    }
}

/// List each function nested in `function`, depth first. `path` locates
/// `function` in the tree: the child indices leading to it, joined by dots.
fn list_children(
    f: &mut Formatter<'_>,
    function: &CompiledFunction,
    path: &str,
    paint: Paint,
) -> fmt::Result {
    for (index, child) in function.child_fns.iter().enumerate() {
        let child_path = child_path(path, index);
        writeln!(f)?;
        list(f, child, Some(&child_path), paint)?;
        list_children(f, child, &child_path, paint)?;
    }
    Ok(())
}

fn child_path(path: &str, index: usize) -> String {
    if path.is_empty() {
        index.to_string()
    } else {
        format!("{path}.{index}")
    }
}

/// List one function; `path` is `None` for the root of the tree.
fn list(
    f: &mut Formatter<'_>,
    function: &CompiledFunction,
    path: Option<&str>,
    paint: Paint,
) -> fmt::Result {
    write!(
        f,
        "{}",
        paint.heading(&format!("function {}", function.name))
    )?;
    match path {
        Some(path) => writeln!(f, " (child {path})")?,
        None => writeln!(f)?,
    }
    writeln!(
        f,
        "  arity {}, {} {}",
        arity(function.arity),
        function.num_captures,
        plural(function.num_captures, "capture", "captures"),
    )?;

    if !function.name_table.is_empty() {
        writeln!(f, "  slots")?;
        for (slot, entry) in function.name_table.iter().enumerate() {
            let mut kinds = Vec::new();
            if slot < function.num_captures {
                kinds.push("capture");
            }
            if entry.exported {
                kinds.push("export");
            }
            let line = format!("    {slot:>4}  {:<12}{}", entry.name, kinds.join(", "));
            writeln!(f, "{}", line.trim_end())?;
        }
    }
    if !function.constants.is_empty() {
        writeln!(f, "  constants")?;
        for (index, constant) in function.constants.iter().enumerate() {
            writeln!(f, "    {index:>4}  {}", constant.to_debug_string())?;
        }
    }
    if !function.key_constants.is_empty() {
        writeln!(f, "  keys")?;
        for (index, key) in function.key_constants.iter().enumerate() {
            writeln!(f, "    {index:>4}  {}", key_value(key).to_debug_string())?;
        }
    }
    if !function.child_fns.is_empty() {
        writeln!(f, "  children")?;
        for (index, child) in function.child_fns.iter().enumerate() {
            let child_path = child_path(path.unwrap_or(""), index);
            writeln!(f, "    {index:>4}  {} (child {child_path})", child.name)?;
        }
    }
    list_code(f, function, paint)
}

fn list_code(f: &mut Formatter<'_>, function: &CompiledFunction, paint: Paint) -> fmt::Result {
    writeln!(f, "  code")?;
    let labels = Labels::of(&function.code);
    let instructions: Vec<Instruction> = (function.code.iter().enumerate())
        .map(|(pc, &op)| instruction(function, &labels, pc, op))
        .collect();
    // Columns as wide as this function needs.
    let name_width = instructions.iter().map(|i| i.name.len()).max().unwrap_or(0) + 2;
    let operand_width = instructions
        .iter()
        .map(|i| i.operand.len())
        .max()
        .unwrap_or(0)
        + 2;
    for (pc, instruction) in instructions.into_iter().enumerate() {
        if let Some(label) = labels.at(pc) {
            writeln!(f, "  {}", paint.label(&format!("L{label}:")))?;
        }
        let Instruction {
            name,
            operand,
            comment,
        } = instruction;
        // Padding goes outside any color, and only before something more.
        write!(f, "    {pc:>4}  {}", paint.opcode(name))?;
        if !operand.is_empty() || comment.is_some() {
            write!(f, "{:1$}", "", name_width - name.len())?;
            write!(f, "{operand}")?;
        }
        if let Some(comment) = comment {
            write!(f, "{:1$}", "", operand_width - operand.len())?;
            write!(f, "{}", paint.comment(&format!("; {comment}")))?;
        }
        writeln!(f)?;
    }
    // A jump may land just past the last instruction, ending the function.
    if let Some(label) = labels.at(function.code.len()) {
        writeln!(f, "  {}", paint.label(&format!("L{label}:")))?;
        writeln!(
            f,
            "    {:>4}  {}",
            function.code.len(),
            paint.comment("(end)")
        )?;
    }
    Ok(())
}

/// One instruction as listed: its opcode's name, its operand, and a comment
/// saying what the operand refers to.
struct Instruction {
    name: &'static str,
    operand: String,
    comment: Option<String>,
}

fn instruction(
    function: &CompiledFunction,
    labels: &Labels,
    pc: usize,
    op: Bytecode,
) -> Instruction {
    use Bytecode::*;

    let bare = |name| (name, String::new(), None);
    let count = |name, n: usize| (name, n.to_string(), None);
    let slot = |name, slot: usize| {
        let named = function
            .name_table
            .get(slot)
            .map(|entry| entry.name.clone());
        (
            name,
            slot.to_string(),
            Some(named.unwrap_or_else(|| "?".into())),
        )
    };
    let key = |name, index: usize| {
        let key = function.key_constants.get(index).map(key_access);
        (
            name,
            index.to_string(),
            Some(key.unwrap_or_else(|| "?".into())),
        )
    };
    let jump = |name, offset: usize| {
        let target = pc + 1 + offset;
        let operand = match labels.at(target) {
            Some(label) if target <= function.code.len() => format!("L{label}"),
            _ => "?".into(),
        };
        let comment = if target <= function.code.len() {
            format!("+{offset}")
        } else {
            format!("+{offset}, past the end")
        };
        (name, operand, Some(comment))
    };

    let (name, operand, comment) = match op {
        PushNull => bare("PushNull"),
        PushTrue => bare("PushTrue"),
        PushFalse => bare("PushFalse"),
        PushInt(i) => ("PushInt", i.to_string(), None),
        PushFloat(float) => ("PushFloat", Value::Float(float).to_debug_string(), None),
        PeekDown(n) => count("PeekDown", n),
        DropBelow(n) => count("DropBelow", n),
        DefLocal(index) => slot("DefLocal", index),
        LoadLocal(index) => slot("LoadLocal", index),
        ConsumeLocal(index) => slot("ConsumeLocal", index),
        LoadConst(index) => {
            let constant = function.constants.get(index).map(inline_constant);
            (
                "LoadConst",
                index.to_string(),
                Some(constant.unwrap_or_else(|| "?".into())),
            )
        }
        LoadGlobal(index) => {
            let global = GLOBAL_NAMES.get(index).copied().unwrap_or("?");
            ("LoadGlobal", index.to_string(), Some(global.to_string()))
        }
        Add => bare("Add"),
        Subtract => bare("Subtract"),
        Multiply => bare("Multiply"),
        Divide => bare("Divide"),
        Modulus => bare("Modulus"),
        CompareEqual => bare("CompareEqual"),
        CompareNotEqual => bare("CompareNotEqual"),
        CompareLessThan => bare("CompareLessThan"),
        CompareLessThanOrEqual => bare("CompareLessThanOrEqual"),
        CompareGreaterThan => bare("CompareGreaterThan"),
        CompareGreaterThanOrEqual => bare("CompareGreaterThanOrEqual"),
        LogicalNot => bare("LogicalNot"),
        Negate => bare("Negate"),
        Concat(n) => count("Concat", n.get()),
        Jump(offset) => jump("Jump", offset),
        JumpIfTrue(offset) => jump("JumpIfTrue", offset),
        JumpIfFalse(offset) => jump("JumpIfFalse", offset),
        PeekJumpIfTrue(offset) => jump("PeekJumpIfTrue", offset),
        PeekJumpIfFalse(offset) => jump("PeekJumpIfFalse", offset),
        Call(argc) => count("Call", argc),
        TailCall(argc) => count("TailCall", argc),
        DynTailCall => bare("DynTailCall"),
        CreateClosure(index) => {
            let child = function.child_fns.get(index).map(|child| {
                format!(
                    "{}, {} {}",
                    child.name,
                    child.num_captures,
                    plural(child.num_captures, "capture", "captures")
                )
            });
            (
                "CreateClosure",
                index.to_string(),
                Some(child.unwrap_or_else(|| "?".into())),
            )
        }
        MakeArray(n) => count("MakeArray", n),
        MakeMap(n) => count("MakeMap", n),
        ExplodeArray => bare("ExplodeArray"),
        SplitArray(n) => count("SplitArray", n),
        SoftIndexStructure => bare("SoftIndexStructure"),
        HardIndexMap(index) => key("HardIndexMap", index),
        TestKey => bare("TestKey"),
        ExtractKey => bare("ExtractKey"),
        TestConstKey(index) => key("TestConstKey", index),
        ExtractConstKey(index) => key("ExtractConstKey", index),
        TypeTest(types) => {
            let types: Vec<String> = types.iter().map(|ty| format!("{ty:?}")).collect();
            ("TypeTest", types.join(" | "), None)
        }
        TestArrayLenExact(n) => count("TestArrayLenExact", n),
        TestArrayLenAtLeast(n) => count("TestArrayLenAtLeast", n),
        MarkStack => bare("MarkStack"),
        DropMark => bare("DropMark"),
        RewindToMark => bare("RewindToMark"),
        ProduceError => bare("ProduceError"),
        Import => bare("Import"),
    };
    Instruction {
        name,
        operand,
        comment,
    }
}

/// The labels of a function's jump targets, numbered in code order.
struct Labels(Vec<usize>);

impl Labels {
    fn of(code: &[Bytecode]) -> Self {
        let mut targets: Vec<usize> = code
            .iter()
            .enumerate()
            .filter_map(|(pc, op)| match *op {
                Bytecode::Jump(offset)
                | Bytecode::JumpIfTrue(offset)
                | Bytecode::JumpIfFalse(offset)
                | Bytecode::PeekJumpIfTrue(offset)
                | Bytecode::PeekJumpIfFalse(offset) => Some(pc + 1 + offset),
                _ => None,
            })
            .filter(|&target| target <= code.len())
            .collect();
        targets.sort_unstable();
        targets.dedup();
        Self(targets)
    }

    /// The label of the target at `pc`, if a jump lands there.
    fn at(&self, pc: usize) -> Option<usize> {
        self.0.binary_search(&pc).ok()
    }
}

/// A constant as shown beside an instruction: cut short if long.
fn inline_constant(constant: &Value) -> String {
    let shown = constant.to_debug_string();
    if shown.chars().count() <= INLINE_CONSTANT_WIDTH {
        return shown;
    }
    let cut: String = shown.chars().take(INLINE_CONSTANT_WIDTH - 3).collect();
    format!("{cut}...")
}

/// A key as Frost source would access it: `.name` where it can, else `[key]`.
fn key_access(key: &MapKey) -> String {
    match key {
        MapKey::String(name) if is_identifier_like_and_not_keyword(name) => format!(".{name}"),
        key => format!("[{}]", key_value(key).to_debug_string()),
    }
}

fn key_value(key: &MapKey) -> Value {
    Value::from(key.clone())
}

fn arity(arity: Arity) -> String {
    match arity {
        Arity::Exact(n) => n.to_string(),
        Arity::AtLeast(n) => format!("{n} or more"),
        Arity::Between(low, high) => format!("{low} to {high}"),
    }
}

fn plural(n: usize, one: &'static str, many: &'static str) -> &'static str {
    if n == 1 { one } else { many }
}

/// Coloring with ANSI escape codes, when on.
#[derive(Clone, Copy)]
struct Paint(bool);

impl Paint {
    fn heading(self, text: &str) -> String {
        self.paint(text, "1")
    }

    fn opcode(self, text: &str) -> String {
        self.paint(text, "36")
    }

    fn label(self, text: &str) -> String {
        self.paint(text, "33")
    }

    fn comment(self, text: &str) -> String {
        self.paint(text, "90")
    }

    fn paint(self, text: &str, code: &str) -> String {
        if self.0 {
            format!("\x1b[{code}m{text}\x1b[0m")
        } else {
            text.to_string()
        }
    }
}
