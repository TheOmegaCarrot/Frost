//! Destructuring: binding a value, or the parts of one, to names.
//!
//! A destructure fragment consumes the value it destructures, `( x -- )`, raising
//! if the value does not have the pattern's shape. Its parts are destructured in
//! source order, each by its own fragment, and each binding is in scope for the
//! parts after it.

use frost_parse::ast::{Binding, Destructure, MapDestructureEntry, Spanned};
use frost_runtime::{Bytecode, FrostArray, FrostType, MapKey, Value};

use crate::{
    CompilerErrors,
    lower::{
        FunctionBuilder, Ir, JumpType, Label, Position,
        fold::constant_of,
        locals::{LocalInfo, LocalKind},
    },
};

/// A compiled destructure, `( x -- )`.
pub(super) struct DestructureFragment {
    pub(super) code: Vec<Ir>,
    /// Whether destructuring uses no runtime input, so a statement made of it
    /// and a foldable value may fold as a whole.
    pub(super) foldable: bool,
}

/// What a failed shape check does.
pub(super) enum Mismatch {
    /// Raise an error with this message.
    Raise(String),
    /// Jump to this label, leaving the stack as the check found it.
    Jump(Label),
}

impl FunctionBuilder<'_> {
    /// Destructure a value into `destructure`'s bindings, each `exported` or not.
    /// `constant` is the value if it is compile-time known; each binding records
    /// its part of it for propagation.
    pub(super) fn compile_destructure(
        &mut self,
        destructure: &Spanned<Destructure>,
        exported: bool,
        constant: Option<Value>,
    ) -> Result<DestructureFragment, CompilerErrors> {
        match &destructure.node {
            Destructure::Binding(binding) => self.compile_binding(binding, exported, constant),
            Destructure::Array { elements, rest } => {
                self.compile_array_destructure(elements, rest.as_ref(), exported, constant)
            }
            Destructure::Map {
                entries,
                bind_whole,
            } => self.compile_map_destructure(entries, bind_whole.as_ref(), exported, constant),
        }
    }

    /// Check the shape of a value: `test` is `( x -- x b )`, and a false result
    /// is handled by `mismatch`. `( x -- x )`.
    pub(super) fn check_shape(
        &mut self,
        test: impl IntoIterator<Item = Ir>,
        mismatch: Mismatch,
    ) -> Vec<Ir> {
        let test = test.into_iter();
        match mismatch {
            Mismatch::Raise(message) => {
                let shaped = self.next_label();
                test.chain([
                    Ir::Jump {
                        kind: JumpType::IfTrue,
                        label: shaped,
                    },
                    Ir::Const(Value::from(message)),
                    Ir::Ready(Bytecode::ProduceError),
                    Ir::Label(shaped),
                ])
                .collect()
            }
            Mismatch::Jump(label) => test
                .chain([Ir::Jump {
                    kind: JumpType::IfFalse,
                    label,
                }])
                .collect(),
        }
    }

    fn compile_binding(
        &mut self,
        binding: &Spanned<Binding>,
        exported: bool,
        constant: Option<Value>,
    ) -> Result<DestructureFragment, CompilerErrors> {
        let store = match &binding.node {
            Binding::Named(name) => {
                let id = self
                    .locals
                    .define(LocalInfo {
                        name: name.clone(),
                        span: binding.span,
                        exported,
                        constant,
                        kind: LocalKind::Binding,
                    })
                    .map_err(|original| self.duplicate_binding(name, binding.span, original))?;
                Ir::DefLocal(id)
            }
            Binding::Discarded => Ir::Ready(Bytecode::Pop),
        };
        Ok(DestructureFragment {
            code: vec![store],
            foldable: true,
        })
    }

    /// Check the value is an Array of `count` elements (at least, if `rest`), then
    /// lay it out for the parts to consume in source order. A rest splits the
    /// Array, its tail beneath the elements; the elements are exploded, the first
    /// on top. `( a -- rest? aN ... a1 a0 )`
    pub(super) fn array_layout(&mut self, count: usize, rest: bool, mismatch: Mismatch) -> Vec<Ir> {
        let test = if rest {
            Bytecode::TestArrayLenAtLeast(count)
        } else {
            Bytecode::TestArrayLenExact(count)
        };
        self.check_shape([Ir::Ready(test)], mismatch)
            .into_iter()
            .chain(rest.then_some(Ir::Ready(Bytecode::SplitArray(count))))
            .chain([Ir::Ready(Bytecode::ExplodeArray)])
            .collect()
    }

    /// `[e1, e2, ...rest]`: see [`array_layout`](Self::array_layout).
    fn compile_array_destructure(
        &mut self,
        elements: &[Spanned<Destructure>],
        rest: Option<&Spanned<Binding>>,
        exported: bool,
        constant: Option<Value>,
    ) -> Result<DestructureFragment, CompilerErrors> {
        let count = elements.len();
        // A known value of the wrong shape raises, so its parts are never bound.
        let known = constant
            .as_ref()
            .and_then(Value::as_array)
            .map(FrostArray::as_slice)
            .filter(|values| match rest {
                Some(_) => values.len() >= count,
                None => values.len() == count,
            });
        let bound = if rest.is_some() {
            "at least"
        } else {
            "exactly"
        };
        let noun = if count == 1 { "element" } else { "elements" };
        let message = format!("Cannot destructure: expected an Array of {bound} {count} {noun}");
        let layout = self.array_layout(count, rest.is_some(), Mismatch::Raise(message));

        let mut parts = elements
            .iter()
            .enumerate()
            .map(|(index, element)| {
                let part = known.map(|values| values[index].clone());
                self.compile_destructure(element, exported, part)
            })
            .collect::<Result<Vec<_>, _>>()?;
        if let Some(rest) = rest {
            let tail = known.map(|values| Value::from(values[count..].to_vec()));
            parts.push(self.compile_binding(rest, exported, tail)?);
        }

        Ok(DestructureFragment {
            foldable: parts.iter().all(|part| part.foldable),
            code: layout
                .into_iter()
                .chain(parts.into_iter().flat_map(|part| part.code))
                .collect(),
        })
    }

    /// `{key: part, ...} as whole`: check the value is a Map, then take each key
    /// in source order, the Map staying beneath. A key is evaluated in place,
    /// `( m -- m k )`, then looked up, `( m k -- m v )`, raising if absent; its
    /// part consumes the value. The Map left at the end is bound whole, or dropped.
    fn compile_map_destructure(
        &mut self,
        entries: &[Spanned<MapDestructureEntry>],
        bind_whole: Option<&Spanned<Binding>>,
        exported: bool,
        constant: Option<Value>,
    ) -> Result<DestructureFragment, CompilerErrors> {
        // A known value that is not a Map raises, so its parts are never bound.
        let known = constant.filter(|value| value.as_map().is_some());
        let mut code = self.check_shape(
            [
                Ir::Ready(Bytecode::Dup),
                Ir::Ready(Bytecode::TypeTest(FrostType::MAP)),
            ],
            Mismatch::Raise("Cannot destructure: expected a Map".to_string()),
        );
        let mut foldable = true;

        for entry in entries {
            // A computed key is folded now, even if the whole statement folds
            // later: a constant key is what lets its part propagate.
            let key = self.compile_expression(&entry.node.key, Position::Inner)?;
            let key = self.fold_if_eligible(key);
            let value = known.as_ref().and_then(Value::as_map).and_then(|map| {
                let key = MapKey::try_from(constant_of(&key.code)?).ok()?;
                map.get(&key).cloned()
            });
            let part = self.compile_destructure(&entry.node.destructure, exported, value)?;
            foldable &= key.foldable && part.foldable;
            code.extend(key.code);
            code.push(Ir::Ready(Bytecode::ExtractKey));
            code.extend(part.code);
        }

        let whole = match bind_whole {
            Some(binding) => self.compile_binding(binding, exported, known)?,
            None => DestructureFragment {
                code: vec![Ir::Ready(Bytecode::Pop)],
                foldable: true,
            },
        };
        code.extend(whole.code);

        Ok(DestructureFragment { code, foldable })
    }
}
