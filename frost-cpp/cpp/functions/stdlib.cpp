#include <frost/stdlib.hpp>

namespace frst
{

namespace
{

// Look up a string key in a Frost Map value.
// Returns nullopt if the value is not a Map or the key is absent.
std::optional<Value_Ptr> map_lookup(const Value_Ptr& v, std::string_view key)
{
    if (not v->is<Map>())
        return std::nullopt;
    auto it = v->raw_get<Map>().find(Value::create(String{key}));
    if (it == v->raw_get<Map>().end())
        return std::nullopt;
    return it->second;
}

} // namespace

// Stage a module into the builder under a two-segment path.
// path.at(0) is the namespace ("std", "ext"), path.at(1) is the module name.
// Modules sharing a namespace are grouped into the same inner Map.
void Stdlib_Registry_Builder::register_module(module_path_t path,
                                              Value_Ptr contents)
{
    if (path.at(0).empty() || path.at(1).empty())
        throw Frost_Interpreter_Error{fmt::format(
            "Empty segment in module path: '{}.{}'", path.at(0), path.at(1))};

    // Get or create the namespace bucket, then check for duplicates
    auto& ns = staged_[std::string{path.at(0)}];
    auto key = Value::create(String{path.at(1)});

    if (ns.contains(key))
        throw Frost_Interpreter_Error{fmt::format(
            "Duplicate module registration: '{}.{}'", path.at(0), path.at(1))};

    ns.emplace(std::move(key), std::move(contents));
}

// Assemble the staged modules into a nested Frost Map and freeze it into an
// immutable Stdlib_Registry. The result is a Map of namespaces, each
// containing a Map of module names to module values:
//   { "std": { "fs": <Map>, "io": <Map>, ... }, "ext": { ... } }
Stdlib_Registry Stdlib_Registry_Builder::build() &&
{
    // staged_ is a flat_map<string, Map> keyed by namespace name.
    // extract() separates it into parallel key and value vectors,
    // allowing us to move from both (flat_map keys are normally const).
    auto [keys, values] = std::move(staged_).extract();

    // Wrap each namespace string and module Map into Frost Values,
    // producing the top-level registry Map.
    // Everything under the top-level namespace (std, ext) is already a
    // fully-formed Map.
    Map root;
    for (auto&& [ns, entries] : std::views::zip(keys, values))
    {
        root.emplace(Value::create(String{std::move(ns)}),
                     Value::create(Value::trusted, std::move(entries)));
    }

    return Stdlib_Registry{Value::create(Value::trusted, std::move(root))};
}

// Walk a dotted path ("std.fs", "std.encoding.b64") through the nested Map.
// This is the C++ equivalent of the Frost prelude's `dig`: a fold over path
// segments, chaining each step through and_then(map_lookup).
// Returns nullopt if any segment is missing or hits a non-Map value,
// which causes import() to fall through to the filesystem search.
std::optional<Value_Ptr> Stdlib_Registry::lookup_module(
    std::string_view path) const
{
    auto segments = std::views::split(path, '.');

    // Starting from the root Map, each iteration tries to descend one level.
    // and_then propagates nullopt on the first failed lookup, so later
    // segments are skipped
    return std::ranges::fold_left(
        segments, std::optional{root_},
        [](std::optional<Value_Ptr> current, auto segment) {
            return current.and_then([&](const Value_Ptr& v) {
                return map_lookup(
                    v, std::string_view{segment.begin(), segment.end()});
            });
        });
}

} // namespace frst
