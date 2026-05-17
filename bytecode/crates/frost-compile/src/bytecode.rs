// Tentative bytecode design -- revisit once Value and runtime foundations are solid.
//
// use frost_core::Value;
//
// #[derive(Clone, Debug)]
// pub struct CompiledFunction {
//     pub instructions: Vec<Instruction>,
//     pub constants: Vec<Value>,
//     pub children: Vec<CompiledFunction>,
//     pub captures: Vec<CaptureSource>,
//     pub arity: Arity,
//     pub local_count: usize,
// }
//
// #[derive(Clone, Copy, Debug)]
// pub enum CaptureSource {
//     Local(usize),
//     Capture(usize),
// }
//
// #[derive(Clone, Copy, Debug)]
// pub enum Arity {
//     Fixed(usize),
//     Variadic { required: usize },
// }
//
// #[derive(Clone, Debug)]
// pub enum Instruction {
//     // Constants
//     Null,
//     True,
//     False,
//     Const(usize),
//
//     // Stack
//     Pop,
//
//     // Locals
//     DefLocal(usize),
//     LoadLocal(usize),
//     LoadCapture(usize),
//
//     // Arithmetic
//     Add,
//     Sub,
//     Mul,
//     Div,
//     Mod,
//     Neg,
//
//     // Comparison
//     Eq,
//     Neq,
//     Lt,
//     Lte,
//     Gt,
//     Gte,
//
//     // Logic
//     Not,
//
//     // Control flow
//     Jump(usize),
//     JumpIfFalsy(usize),
//     JumpIfTruthy(usize),
//
//     // Functions
//     Call(usize),
//     TailCall(usize),
//     MakeClosure(usize),
//     Return,
//
//     // Data structures
//     MakeArray(usize),
//     MakeMap(usize),
//     Index,
//
//     // Iterative
//     Map,
//     Filter,
//     Reduce,
//     Foreach,
// }
