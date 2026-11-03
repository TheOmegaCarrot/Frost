//! [`Disassembly`]: a readable listing of a [`CompiledFunction`] tree, rendered
//! from its [`Listing`].

use std::fmt::{self, Display, Formatter};

use crate::bytecode::{
    ArrayPatternLength, Bytecode, CompiledFunction, CompiledPattern, GLOBAL_NAMES, ListedFunction,
    ListedInstruction, Listing, PatternComparand, PatternInput, PatternKey, Reference,
};
use crate::core::util::count::counted;
use crate::core::util::identifier::is_identifier_like_and_not_keyword;
use crate::vm::expected_types;
use crate::vm::listing::entry;
use crate::{Arity, MapKey, Value};

/// The widest a constant or pattern shown beside an instruction may be before
/// it is cut short. The function's own list of each shows them in full.
const INLINE_WIDTH: usize = 40;

impl CompiledFunction {
    /// A readable listing of this function, then of each function nested in
    /// it, depth first. Print it with `{}`.
    ///
    /// Each listing shows the function's arity, its slots, constants, Map keys,
    /// patterns, and nested functions, then its code. An instruction with an
    /// alias, such as [`Bytecode::Dup`], lists by the alias. Jumps name labeled
    /// targets, and each instruction that refers to a slot, constant, key,
    /// pattern, global, or nested function is annotated with what it refers
    /// to. Where the function has debug info, each instruction also shows its
    /// source location and its entry stack height.
    ///
    /// The listing is meant for people, and its format may change; for the same
    /// information as data, see [`listing`](Self::listing). Malformed bytecode
    /// lists without panicking: a reference to nothing shows as `?`.
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
        let Listing { functions } = self.function.listing();
        for (index, function) in functions.iter().enumerate() {
            if index > 0 {
                writeln!(f)?;
            }
            list(f, function, paint)?;
        }
        Ok(())
    }
}

/// `path` as listed: the indices joined by dots.
fn dotted(path: &[usize]) -> String {
    path.iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join(".")
}

/// List one function.
fn list(f: &mut Formatter<'_>, function: &ListedFunction, paint: Paint) -> fmt::Result {
    write!(
        f,
        "{}",
        paint.heading(&format!("function {}", function.name))
    )?;
    if function.path.is_empty() {
        writeln!(f)?;
    } else {
        writeln!(f, " (child {})", dotted(&function.path))?;
    }
    writeln!(
        f,
        "  arity {}, {}",
        arity(function.arity),
        counted(function.num_captures, "capture"),
    )?;

    if !function.slots.is_empty() {
        writeln!(f, "  slots")?;
        let name_width = function
            .slots
            .iter()
            .map(|slot| slot.name.chars().count())
            .max()
            .unwrap_or(0);
        for (index, slot) in function.slots.iter().enumerate() {
            let kinds: Vec<&str> = [(slot.capture, "capture"), (slot.exported, "export")]
                .into_iter()
                .filter_map(|(is, kind)| is.then_some(kind))
                .collect();
            let line = format!(
                "    {index:>4}  {:<name_width$}  {}",
                slot.name,
                kinds.join(", ")
            );
            writeln!(f, "{}", line.trim_end())?;
        }
    }
    if !function.constants.is_empty() {
        writeln!(f, "  constants")?;
        for (index, constant) in function.constants.iter().enumerate() {
            writeln!(f, "    {index:>4}  {}", constant.to_debug_string())?;
        }
    }
    if !function.keys.is_empty() {
        writeln!(f, "  keys")?;
        for (index, key) in function.keys.iter().enumerate() {
            writeln!(f, "    {index:>4}  {}", key_value(key).to_debug_string())?;
        }
    }
    if !function.patterns.is_empty() {
        writeln!(f, "  patterns")?;
        for (index, pattern) in function.patterns.iter().enumerate() {
            writeln!(f, "    {index:>4}  {}", pattern_source(pattern, function))?;
        }
    }
    if !function.children.is_empty() {
        writeln!(f, "  children")?;
        for (index, name) in function.children.iter().enumerate() {
            let child_path: Vec<usize> = function.path.iter().copied().chain([index]).collect();
            writeln!(f, "    {index:>4}  {name} (child {})", dotted(&child_path))?;
        }
    }
    list_code(f, function, paint)
}

/// One instruction as listed: its debug columns, if the function has debug
/// info, then its opcode's name, its operand, and a comment saying what the
/// operand refers to.
struct Row {
    debug: Option<(String, String)>,
    name: &'static str,
    operand: String,
    comment: Option<String>,
}

fn list_code(f: &mut Formatter<'_>, function: &ListedFunction, paint: Paint) -> fmt::Result {
    let has_debug_info = function
        .instructions
        .iter()
        .any(|instruction| instruction.debug.is_some());
    if has_debug_info {
        writeln!(f, "  code (line:column, stack height)")?;
    } else {
        writeln!(f, "  code")?;
    }
    let rows: Vec<Row> = function
        .instructions
        .iter()
        .map(|instruction| row(instruction, function, has_debug_info))
        .collect();
    // Columns as wide as this function needs.
    let width = |of: &dyn Fn(&Row) -> usize| rows.iter().map(of).max().unwrap_or(0);
    let location_width = width(&|row| row.debug.as_ref().map_or(0, |(at, _)| at.len()));
    let height_width = width(&|row| row.debug.as_ref().map_or(0, |(_, height)| height.len()));
    let name_width = width(&|row| row.name.len()) + 2;
    let operand_width = width(&|row| row.operand.len()) + 2;
    for (instruction, row) in function.instructions.iter().zip(rows) {
        if let Some(label) = instruction.label {
            writeln!(f, "  {}", paint.label(&format!("L{label}:")))?;
        }
        let Row {
            debug,
            name,
            operand,
            comment,
        } = row;
        write!(f, "    {:>4}  ", instruction.index)?;
        if let Some((at, height)) = debug {
            write!(f, "{at:<location_width$}  {height:>height_width$}  ")?;
        }
        // Padding goes outside any color, and only before something more.
        write!(f, "{}", paint.opcode(name))?;
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
    if let Some(label) = function.end_label {
        writeln!(f, "  {}", paint.label(&format!("L{label}:")))?;
        write!(f, "    {:>4}  ", function.instructions.len())?;
        // Under the opcodes, past any debug columns, which it has none of.
        if has_debug_info {
            write!(f, "{:location_width$}  {:height_width$}  ", "", "")?;
        }
        writeln!(f, "{}", paint.comment("(end)"))?;
    }
    Ok(())
}

/// How `instruction`, of `function`, lists; with debug columns if
/// `has_debug_info`, empty where it has no entry and `-` for an unknown height.
fn row(instruction: &ListedInstruction, function: &ListedFunction, has_debug_info: bool) -> Row {
    let debug = has_debug_info.then(|| match instruction.debug {
        Some(debug) => (
            format!("{}:{}", debug.location.line(), debug.location.column()),
            debug
                .op
                .stack_height
                .map_or_else(|| "-".to_string(), |height| height.to_string()),
        ),
        None => (String::new(), String::new()),
    });
    let (operand, comment) = match instruction.reference {
        Some(reference) => (
            reference_operand(reference),
            Some(reference_comment(reference, function)),
        ),
        None => (operand(instruction.op), None),
    };
    Row {
        debug,
        name: opcode_name(instruction.op),
        operand,
        comment,
    }
}

/// The operand of an instruction that refers to something.
fn reference_operand(reference: Reference) -> String {
    match reference {
        Reference::Slot { index, .. }
        | Reference::Constant { index, .. }
        | Reference::Key { index, .. }
        | Reference::Pattern { index, .. }
        | Reference::Global { index, .. } => index.to_string(),
        Reference::Child { index, .. } => index.to_string(),
        Reference::Slots { first, count, .. } => {
            format!("{first}..{}", u64::from(first) + u64::from(count))
        }
        Reference::Jump { target, .. } => {
            target.map_or_else(|| "?".to_string(), |target| format!("L{}", target.label))
        }
    }
}

/// What a referring instruction's operand refers to in `function`, as its comment.
fn reference_comment(reference: Reference, function: &ListedFunction) -> String {
    match reference {
        Reference::Slot { name, .. } | Reference::Global { name, .. } => {
            name.unwrap_or("?").to_string()
        }
        // Before the empty run, so one past the name table lists as unknown.
        Reference::Slots { names: None, .. } => "?".to_string(),
        Reference::Slots { count: 0, .. } => "no slots".to_string(),
        Reference::Slots {
            names: Some(names), ..
        } => names.iter().collect::<Vec<_>>().join(", "),
        Reference::Constant { value, .. } => value.map_or_else(
            || "?".to_string(),
            |value| cut_short(value.to_debug_string()),
        ),
        Reference::Key { key, .. } => key.map_or_else(|| "?".to_string(), key_access),
        Reference::Pattern { pattern, .. } => pattern.map_or_else(
            || "?".to_string(),
            |pattern| cut_short(pattern_source(pattern, function)),
        ),
        Reference::Child { name, captures, .. } => {
            format!("{}, {}", name.unwrap_or("?"), counted(captures, "capture"))
        }
        Reference::Jump { offset, target } => match target {
            Some(_) => format!("+{offset}"),
            None => format!("+{offset}, past the end"),
        },
    }
}

/// The operand of an instruction that refers to nothing: a count or a literal,
/// or nothing.
fn operand(op: Bytecode) -> String {
    use Bytecode::*;

    // Exhaustive, so that a new opcode decides how its operand lists.
    match op {
        PushInt(int) => int.to_string(),
        PushFloat(float) => Value::Float(float).to_debug_string(),
        // At 0, these are `Dup` and `Pop`, which list by name alone.
        PeekDown(n @ 1..)
        | DropBelow(n @ 1..)
        | Call(n)
        | TailCall(n)
        | MakeArray(n)
        | MakeMap(n)
        | ExplodeArray(n)
        | SplitArray(n)
        | TestArrayLenExact(n)
        | TestArrayLenAtLeast(n) => n.to_string(),
        Concat(n) => n.to_string(),
        TypeTest(types) => {
            let types: Vec<String> = types.iter().map(|ty| format!("{ty:?}")).collect();
            types.join(" | ")
        }
        // No operand, or (from `DefLocal` on) one that refers to something, so
        // lists by its reference instead.
        Bytecode::Dup
        | Bytecode::Pop
        | PushNull
        | PushTrue
        | PushFalse
        | Add
        | Subtract
        | Multiply
        | Divide
        | Modulus
        | CompareEqual
        | CompareNotEqual
        | CompareLessThan
        | CompareLessThanOrEqual
        | CompareGreaterThan
        | CompareGreaterThanOrEqual
        | LogicalNot
        | Negate
        | DynTailCall
        | SoftIndexStructure
        | TestKey
        | ExtractKey
        | MarkStack
        | DropMark
        | RewindToMark
        | ProduceError
        | Import
        | DefLocal(_)
        | LoadLocal(_)
        | ConsumeLocal(_)
        | ClearLocals { .. }
        | LoadConst(_)
        | LoadGlobal(_)
        | HardIndexMap(_)
        | TestConstKey(_)
        | ExtractConstKey(_)
        | MatchPattern(_)
        | CreateClosure { .. }
        | Jump(_)
        | JumpIfTrue(_)
        | JumpIfFalse(_)
        | PeekJumpIfTrue(_)
        | PeekJumpIfFalse(_) => String::new(),
    }
}

/// The name an opcode lists by: its alias's, if it has one.
fn opcode_name(op: Bytecode) -> &'static str {
    use Bytecode::*;

    match op {
        Bytecode::Dup => "Dup",
        Bytecode::Pop => "Pop",
        Bytecode::Nop => "Nop",
        PushNull => "PushNull",
        PushTrue => "PushTrue",
        PushFalse => "PushFalse",
        PushInt(_) => "PushInt",
        PushFloat(_) => "PushFloat",
        PeekDown(_) => "PeekDown",
        DropBelow(_) => "DropBelow",
        DefLocal(_) => "DefLocal",
        LoadLocal(_) => "LoadLocal",
        ConsumeLocal(_) => "ConsumeLocal",
        ClearLocals { .. } => "ClearLocals",
        LoadConst(_) => "LoadConst",
        LoadGlobal(_) => "LoadGlobal",
        Add => "Add",
        Subtract => "Subtract",
        Multiply => "Multiply",
        Divide => "Divide",
        Modulus => "Modulus",
        CompareEqual => "CompareEqual",
        CompareNotEqual => "CompareNotEqual",
        CompareLessThan => "CompareLessThan",
        CompareLessThanOrEqual => "CompareLessThanOrEqual",
        CompareGreaterThan => "CompareGreaterThan",
        CompareGreaterThanOrEqual => "CompareGreaterThanOrEqual",
        LogicalNot => "LogicalNot",
        Negate => "Negate",
        Concat(_) => "Concat",
        Jump(_) => "Jump",
        JumpIfTrue(_) => "JumpIfTrue",
        JumpIfFalse(_) => "JumpIfFalse",
        PeekJumpIfTrue(_) => "PeekJumpIfTrue",
        PeekJumpIfFalse(_) => "PeekJumpIfFalse",
        Call(_) => "Call",
        TailCall(_) => "TailCall",
        DynTailCall => "DynTailCall",
        CreateClosure { .. } => "CreateClosure",
        MakeArray(_) => "MakeArray",
        MakeMap(_) => "MakeMap",
        ExplodeArray(_) => "ExplodeArray",
        SplitArray(_) => "SplitArray",
        SoftIndexStructure => "SoftIndexStructure",
        HardIndexMap(_) => "HardIndexMap",
        TestKey => "TestKey",
        ExtractKey => "ExtractKey",
        TestConstKey(_) => "TestConstKey",
        ExtractConstKey(_) => "ExtractConstKey",
        TypeTest(_) => "TypeTest",
        TestArrayLenExact(_) => "TestArrayLenExact",
        TestArrayLenAtLeast(_) => "TestArrayLenAtLeast",
        MatchPattern(_) => "MatchPattern",
        MarkStack => "MarkStack",
        DropMark => "DropMark",
        RewindToMark => "RewindToMark",
        ProduceError => "ProduceError",
        Import => "Import",
    }
}

/// A constant or pattern as shown beside an instruction: cut short if long.
fn cut_short(shown: String) -> String {
    if shown.chars().count() <= INLINE_WIDTH {
        return shown;
    }
    let cut: String = shown.chars().take(INLINE_WIDTH - 3).collect();
    format!("{cut}...")
}

/// `pattern`, of `function`, written much as Frost source would write it, with
/// what it refers to by name or value, and `?` for what it refers to that is
/// not there.
fn pattern_source(pattern: &CompiledPattern, function: &ListedFunction) -> String {
    match pattern {
        CompiledPattern::Binding { types, local } => {
            let name = local.map_or("_", |local| slot_name(function, local));
            match types {
                None => name.to_string(),
                Some(types) => format!("{name} is {}", expected_types(*types)),
            }
        }
        CompiledPattern::Value(comparand) => comparand_source(comparand, function),
        CompiledPattern::Array {
            elements,
            length,
            whole,
        } => {
            let rest = match length {
                ArrayPatternLength::Exact => None,
                ArrayPatternLength::AtLeast { rest } => Some(format!(
                    "...{}",
                    rest.map_or("_", |rest| slot_name(function, rest))
                )),
            };
            let parts: Vec<String> = elements
                .iter()
                .map(|element| pattern_source(element, function))
                .chain(rest)
                .collect();
            with_whole(format!("[{}]", parts.join(", ")), *whole, function)
        }
        CompiledPattern::Map { entries, whole } => {
            let entries: Vec<String> = entries
                .iter()
                .map(|entry| {
                    format!(
                        "{}{}: {}",
                        key_source(&entry.key, function),
                        if entry.optional { "?" } else { "" },
                        pattern_source(&entry.pattern, function)
                    )
                })
                .collect();
            with_whole(format!("{{{}}}", entries.join(", ")), *whole, function)
        }
        // An Alternative that is a branch of another lists merged into its
        // parent's branches, without parentheses; that matches the same, since
        // the first branch to match commits.
        CompiledPattern::Alternative(branches) => {
            let branches: Vec<String> = branches
                .iter()
                .map(|branch| pattern_source(branch, function))
                .collect();
            branches.join(" | ")
        }
    }
}

/// `pattern`, an Array or Map pattern's source, followed by its `as` binding
/// to `whole`, if any.
fn with_whole(pattern: String, whole: Option<u32>, function: &ListedFunction) -> String {
    match whole {
        None => pattern,
        Some(whole) => format!("{pattern} as {}", slot_name(function, whole)),
    }
}

/// A Map pattern's `key`, as it is written before the colon.
fn key_source(key: &PatternKey, function: &ListedFunction) -> String {
    match key {
        PatternKey::Constant(key) => entry(function.keys, u64::from(*key)).map_or_else(
            || "?".to_string(),
            |key| bare_key(key).map_or_else(|| bracketed_key(key), str::to_string),
        ),
        PatternKey::Input(input) => format!("[{}]", input_name(input, function)),
    }
}

/// What a value pattern compares against: a literal, or a pin.
fn comparand_source(comparand: &PatternComparand, function: &ListedFunction) -> String {
    match comparand {
        PatternComparand::Null => Value::Null.to_debug_string(),
        PatternComparand::Bool(bool) => Value::Bool(*bool).to_debug_string(),
        PatternComparand::Int(int) => Value::Int(*int).to_debug_string(),
        PatternComparand::Float(float) => Value::Float(*float).to_debug_string(),
        PatternComparand::Constant(constant) => entry(function.constants, u64::from(*constant))
            .map_or_else(|| "?".to_string(), Value::to_debug_string),
        PatternComparand::Input(input) => format!("({})", input_name(input, function)),
    }
}

/// The name of what a pattern takes from outside the value it tests.
fn input_name<'a>(input: &PatternInput, function: &ListedFunction<'a>) -> &'a str {
    match *input {
        PatternInput::Local(local) | PatternInput::EarlierBinding(local) => {
            slot_name(function, local)
        }
        PatternInput::Global(global) => entry(GLOBAL_NAMES, u64::from(global))
            .copied()
            .unwrap_or("?"),
    }
}

/// The name of `function`'s slot `local`.
fn slot_name<'a>(function: &ListedFunction<'a>, local: u32) -> &'a str {
    entry(&function.slots, u64::from(local)).map_or("?", |slot| slot.name)
}

/// A key as Frost source would access it: `.name` where it can, else `[key]`.
fn key_access(key: &MapKey) -> String {
    bare_key(key).map_or_else(|| bracketed_key(key), |name| format!(".{name}"))
}

/// `key`'s name, if source can write it bare: `.name` to access it, or `name:`
/// in a Map.
fn bare_key(key: &MapKey) -> Option<&str> {
    match key {
        MapKey::String(name) if is_identifier_like_and_not_keyword(name) => Some(name),
        _ => None,
    }
}

/// `key` in brackets, as source writes a key it cannot write bare.
fn bracketed_key(key: &MapKey) -> String {
    format!("[{}]", key_value(key).to_debug_string())
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
