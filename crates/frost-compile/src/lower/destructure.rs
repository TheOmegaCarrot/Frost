//! Destructuring: binding a value, or the parts of one, to names.
//!
//! A destructure fragment consumes the value it destructures, `( x -- )`, raising
//! if the value does not have the pattern's shape. Its parts are destructured in
//! source order, each by its own fragment, and each binding is in scope for the
//! parts after it.

use frost_parse::ast::{Binding, Destructure, MapDestructureEntry, Spanned};
use frost_runtime::{Bytecode, FrostType, Value};

use crate::{
    CompilerErrors,
    lower::{
        FunctionBuilder, Ir, JumpType, Position,
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

impl FunctionBuilder<'_> {
    /// Destructure a value into `destructure`'s bindings, each `exported` or not.
    /// `constant` is the value if it is compile-time known; a plain binding
    /// records it for propagation.
    pub(super) fn compile_destructure(
        &mut self,
        destructure: &Spanned<Destructure>,
        exported: bool,
        constant: Option<Value>,
    ) -> Result<DestructureFragment, CompilerErrors> {
        match &destructure.node {
            Destructure::Binding(binding) => self.compile_binding(binding, exported, constant),
            // TODO: record a constant for each part of a compile-time-known value,
            // so destructured bindings propagate too.
            Destructure::Array { elements, rest } => {
                self.compile_array_destructure(elements, rest.as_ref(), exported)
            }
            Destructure::Map {
                entries,
                bind_whole,
            } => self.compile_map_destructure(entries, bind_whole.as_ref(), exported),
        }
    }

    /// Check the shape of the value being destructured: `test` is `( x -- x b )`,
    /// and a false result raises `message`. `( x -- x )`.
    fn check_shape(&mut self, test: impl IntoIterator<Item = Ir>, message: String) -> Vec<Ir> {
        let shaped = self.next_label();
        test.into_iter()
            .chain([
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

    /// `[e1, e2, ...rest]`: check the value is an Array of the right length, then
    /// lay it out for the parts to consume in source order. A rest splits the
    /// Array, its tail beneath the elements; the elements are exploded, the first
    /// on top.
    fn compile_array_destructure(
        &mut self,
        elements: &[Spanned<Destructure>],
        rest: Option<&Spanned<Binding>>,
        exported: bool,
    ) -> Result<DestructureFragment, CompilerErrors> {
        let count = elements.len();
        let (test, bound) = match rest {
            None => (Bytecode::TestArrayLenExact(count), "exactly"),
            Some(_) => (Bytecode::TestArrayLenAtLeast(count), "at least"),
        };
        let noun = if count == 1 { "element" } else { "elements" };
        let message = format!("Cannot destructure: expected an Array of {bound} {count} {noun}");

        let layout = self
            .check_shape([Ir::Ready(test)], message)
            .into_iter()
            .chain(rest.map(|_| Ir::Ready(Bytecode::SplitArray(count))))
            .chain([Ir::Ready(Bytecode::ExplodeArray)]);

        let mut parts = elements
            .iter()
            .map(|element| self.compile_destructure(element, exported, None))
            .collect::<Result<Vec<_>, _>>()?;
        if let Some(rest) = rest {
            parts.push(self.compile_binding(rest, exported, None)?);
        }

        Ok(DestructureFragment {
            foldable: parts.iter().all(|part| part.foldable),
            code: layout
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
    ) -> Result<DestructureFragment, CompilerErrors> {
        let mut code = self.check_shape(
            [
                Ir::Ready(Bytecode::Dup),
                Ir::Ready(Bytecode::TypeTest(FrostType::MAP)),
            ],
            "Cannot destructure: expected a Map".to_string(),
        );
        let mut foldable = true;

        for entry in entries {
            // A computed key is folded now, even if the whole statement folds
            // later: a constant key is what lets its part propagate.
            let key = self.compile_expression(&entry.node.key, Position::Inner)?;
            let key = self.fold_if_eligible(key);
            let part = self.compile_destructure(&entry.node.destructure, exported, None)?;
            foldable &= key.foldable && part.foldable;
            code.extend(key.code);
            code.push(Ir::Ready(Bytecode::ExtractKey));
            code.extend(part.code);
        }

        let whole = match bind_whole {
            Some(binding) => self.compile_binding(binding, exported, None)?,
            None => DestructureFragment {
                code: vec![Ir::Ready(Bytecode::Pop)],
                foldable: true,
            },
        };
        code.extend(whole.code);

        Ok(DestructureFragment { code, foldable })
    }
}
