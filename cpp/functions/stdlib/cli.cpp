#include <frost/builtins-common.hpp>

#include <frost/value.hpp>

#include <flat_map>
#include <flat_set>

namespace frst
{

namespace cli
{

namespace
{

struct Flag_Spec
{
    String long_name;
    std::optional<char> short_name;
    std::optional<String> description;
};

struct Option_Spec
{
    String long_name;
    std::optional<char> short_name;
    bool required = false;
    std::optional<String> default_value;
    bool repeatable = false;
    std::optional<String> description;
};

struct Positional_Spec
{
    String name;
    std::optional<String> description;
};

struct Cli_Spec
{
    std::optional<String> name;
    std::optional<String> description;
    std::optional<String> help_text;
    std::vector<Flag_Spec> flags;
    std::vector<Option_Spec> options;
    std::vector<Positional_Spec> positional;
    bool raw_collect_positional = false;

    std::flat_map<char, std::size_t> short_to_flag;
    std::flat_map<char, std::size_t> short_to_option;
    std::flat_map<String, std::size_t> long_to_flag;
    std::flat_map<String, std::size_t> long_to_option;
};

// Type-check helper: throws with a contextual message or returns a
// reference to the underlying value (or a copy, for trivially-copyable
// types like Bool).
template <Frost_Type T>
const auto& require_as(const Value_Ptr& v, std::string_view what)
{
    if (not v->is<T>())
        throw Frost_Recoverable_Error{
            fmt::format("cli.parse: {} must be a {}, got {}", what,
                        type_str<T>(), v->type_name())};
    return v->raw_get<T>();
}

// Parses a short-name spec: must be a single-character String.
char require_short_char(const Value_Ptr& v, std::string_view what)
{
    const auto& s = require_as<String>(v, fmt::format("{}: short", what));
    if (s.size() != 1)
        throw Frost_Recoverable_Error{fmt::format(
            "cli.parse: {}: short must be a single character", what)};
    return s.at(0);
}

Flag_Spec parse_one_flag(const String& long_name, const Map& sub)
{
    Flag_Spec result;
    result.long_name = long_name;
    auto ctx = fmt::format("flag '{}'", long_name);

    for (const auto& [k_val, v_val] : sub)
    {
        const auto& key = require_as<String>(k_val, "flag spec key");

        if (key == "short")
            result.short_name = require_short_char(v_val, ctx);
        else if (key == "description")
            result.description =
                require_as<String>(v_val, fmt::format("{}: description", ctx));
        else
            throw Frost_Recoverable_Error{
                fmt::format("cli.parse: {}: unknown key '{}'", ctx, key)};
    }

    return result;
}

std::vector<Flag_Spec> parse_flags(const Map& flags_map)
{
    std::vector<Flag_Spec> result;

    for (const auto& [k_val, v_val] : flags_map)
    {
        const auto& long_name = require_as<String>(k_val, "flags key");
        const auto& sub =
            require_as<Map>(v_val, fmt::format("flag '{}' spec", long_name));
        result.push_back(parse_one_flag(long_name, sub));
    }

    return result;
}

Option_Spec parse_one_option(const String& long_name, const Map& sub)
{
    Option_Spec result;
    result.long_name = long_name;
    auto ctx = fmt::format("option '{}'", long_name);

    for (const auto& [k_val, v_val] : sub)
    {
        const auto& key = require_as<String>(k_val, "option spec key");

        if (key == "short")
            result.short_name = require_short_char(v_val, ctx);
        else if (key == "required")
            result.required =
                require_as<Bool>(v_val, fmt::format("{}: required", ctx));
        else if (key == "default")
            result.default_value =
                require_as<String>(v_val, fmt::format("{}: default", ctx));
        else if (key == "repeatable")
            result.repeatable =
                require_as<Bool>(v_val, fmt::format("{}: repeatable", ctx));
        else if (key == "description")
            result.description =
                require_as<String>(v_val, fmt::format("{}: description", ctx));
        else
            throw Frost_Recoverable_Error{
                fmt::format("cli.parse: {}: unknown key '{}'", ctx, key)};
    }

    if (result.required && result.default_value)
        throw Frost_Recoverable_Error{fmt::format(
            "cli.parse: {}: required and default are mutually exclusive", ctx)};

    return result;
}

std::vector<Option_Spec> parse_options(const Map& options_map)
{
    std::vector<Option_Spec> result;

    for (const auto& [k_val, v_val] : options_map)
    {
        const auto& long_name = require_as<String>(k_val, "options key");
        const auto& sub =
            require_as<Map>(v_val, fmt::format("option '{}' spec", long_name));
        result.push_back(parse_one_option(long_name, sub));
    }

    return result;
}

// Returns the parsed positional specs.
// For `positional: true`, the caller handles the raw-collect flag directly.
std::vector<Positional_Spec> parse_positional(const Value_Ptr& val)
{
    if (not val->is<Array>())
        throw Frost_Recoverable_Error{
            "cli.parse: positional must be true or an Array"};

    std::vector<Positional_Spec> result;
    const auto& arr = val->raw_get<Array>();

    for (const auto& [i, entry] : std::views::enumerate(arr))
    {
        auto ctx = fmt::format("positional[{}]", i);
        const auto& entry_map = require_as<Map>(entry, ctx);

        Positional_Spec spec;
        bool has_name = false;

        for (const auto& [k_val, v_val] : entry_map)
        {
            const auto& key = require_as<String>(k_val, "positional spec key");

            if (key == "name")
            {
                spec.name =
                    require_as<String>(v_val, fmt::format("{}: name", ctx));
                has_name = true;
            }
            else if (key == "description")
                spec.description = require_as<String>(
                    v_val, fmt::format("{}: description", ctx));
            else
                throw Frost_Recoverable_Error{
                    fmt::format("cli.parse: {}: unknown key '{}'", ctx, key)};
        }

        if (not has_name)
            throw Frost_Recoverable_Error{
                fmt::format("cli.parse: {}: name is required", ctx)};

        result.push_back(std::move(spec));
    }

    return result;
}

Cli_Spec parse_spec(const Map& spec_map)
{
    Cli_Spec spec;

    for (const auto& [k_val, v_val] : spec_map)
    {
        const auto& key = require_as<String>(k_val, "spec key");

        if (key == "name")
            spec.name = require_as<String>(v_val, "name");
        else if (key == "description")
            spec.description = require_as<String>(v_val, "description");
        else if (key == "help")
            spec.help_text = require_as<String>(v_val, "help");
        else if (key == "flags")
            spec.flags = parse_flags(require_as<Map>(v_val, "flags"));
        else if (key == "options")
            spec.options = parse_options(require_as<Map>(v_val, "options"));
        else if (key == "positional")
        {
            if (v_val->is<Bool>() && v_val->raw_get<Bool>())
                spec.raw_collect_positional = true;
            else
                spec.positional = parse_positional(v_val);
        }
        else
            throw Frost_Recoverable_Error{
                fmt::format("cli.parse: unknown spec key '{}'", key)};
    }

    return spec;
}

void validate_spec(Cli_Spec& spec)
{
    // Shared sets detect collisions across flags AND options --
    // a flag and option cannot share a long name or short name.
    std::flat_set<char> all_shorts;
    std::flat_set<String> all_longs;

    // Register flags: check uniqueness, build lookup tables.
    // Index loop because views::enumerate is doing something weird
    for (std::size_t i = 0; i < spec.flags.size(); ++i)
    {
        const auto& f = spec.flags.at(i);

        if (not all_longs.insert(f.long_name).second)
            throw Frost_Recoverable_Error{
                fmt::format("cli.parse: duplicate name '{}'", f.long_name)};

        spec.long_to_flag.insert_or_assign(f.long_name, i);

        if (f.short_name)
        {
            if (not all_shorts.insert(f.short_name.value()).second)
                throw Frost_Recoverable_Error{
                    fmt::format("cli.parse: duplicate short name '-{}'",
                                f.short_name.value())};
            spec.short_to_flag.insert_or_assign(f.short_name.value(), i);
        }
    }

    // Register options: same structure, different lookup maps
    // (index loop for the same reason as above).
    for (std::size_t i = 0; i < spec.options.size(); ++i)
    {
        const auto& o = spec.options.at(i);
        if (not all_longs.insert(o.long_name).second)
            throw Frost_Recoverable_Error{
                fmt::format("cli.parse: duplicate name '{}'", o.long_name)};

        spec.long_to_option.insert_or_assign(o.long_name, i);

        if (o.short_name)
        {
            if (not all_shorts.insert(o.short_name.value()).second)
                throw Frost_Recoverable_Error{
                    fmt::format("cli.parse: duplicate short name '-{}'",
                                o.short_name.value())};
            spec.short_to_option.insert_or_assign(o.short_name.value(), i);
        }
    }
}

String generate_help(const Cli_Spec& spec, const String& tool_name);

Value_Ptr parse_args(const Cli_Spec& spec, const Array& args,
                     const String& tool_name)
{
    // Parse state: vectors indexed parallel to spec.flags and
    // spec.options, so lookup tables can feed their indices directly in.
    std::vector<bool> flag_values(spec.flags.size(), false);
    std::vector<std::vector<String>> option_values(spec.options.size());

    std::vector<String> positionals;
    bool past_double_dash = false;

    auto err = [&](std::string_view msg) {
        throw Frost_Recoverable_Error{fmt::format("{}: {}", tool_name, msg)};
    };

    // Index-based loop: args[0] is skipped, and we sometimes advance `i`
    // mid-iteration to consume the next arg as an option value.
    for (std::size_t i = 1; i < args.size(); ++i)
    {
        const auto& arg = args.at(i)->raw_get<String>();

        // After --, or not starting with -, or bare "-": positional
        if (past_double_dash || arg.empty() || arg.at(0) != '-' || arg == "-")
        {
            positionals.push_back(arg);
            continue;
        }

        // "--" terminates flag parsing
        if (arg == "--")
        {
            past_double_dash = true;
            continue;
        }

        // Long form: --name
        if (arg.starts_with("--"))
        {
            auto name = arg.substr(2);

            auto fit = spec.long_to_flag.find(name);
            if (fit != spec.long_to_flag.end())
            {
                flag_values.at(fit->second) = true;
                continue;
            }

            auto oit = spec.long_to_option.find(name);
            if (oit != spec.long_to_option.end())
            {
                const auto& opt = spec.options.at(oit->second);
                auto& vals = option_values.at(oit->second);
                if (i + 1 >= args.size())
                    err(fmt::format("option '--{}' requires a value", name));
                ++i;
                if (not vals.empty() && not opt.repeatable)
                    err(fmt::format("option '--{}' cannot be repeated", name));
                vals.push_back(args.at(i)->raw_get<String>());
                continue;
            }

            err(fmt::format("unknown option '--{}'", name));
        }
        else
        {
            // Short form: -abc
            // Index loop because a value-taking option must be the last char
            // in the bundle, and we need the position to enforce that.
            for (std::size_t j = 1; j < arg.size(); ++j)
            {
                char c = arg.at(j);

                auto fit = spec.short_to_flag.find(c);
                if (fit != spec.short_to_flag.end())
                {
                    flag_values.at(fit->second) = true;
                    continue;
                }

                auto oit = spec.short_to_option.find(c);
                if (oit != spec.short_to_option.end())
                {
                    // Value-taking option must be the last char in a bundle.
                    if (j + 1 < arg.size())
                        err(fmt::format(
                            "option '-{}' takes a value and must be the last "
                            "option in a bundle",
                            c));
                    const auto& opt = spec.options.at(oit->second);
                    auto& vals = option_values.at(oit->second);
                    if (i + 1 >= args.size())
                        err(fmt::format("option '-{}' requires a value", c));
                    ++i;
                    if (not vals.empty() && not opt.repeatable)
                        err(fmt::format("option '-{}' cannot be repeated", c));
                    vals.push_back(args.at(i)->raw_get<String>());
                    break; // value-taking option ends the bundle
                }

                err(fmt::format("unknown option '-{}'", c));
            }
        }
    }

    // Post-walk: check required options and apply defaults
    for (std::size_t idx = 0; idx < spec.options.size(); ++idx)
    {
        const auto& o = spec.options.at(idx);
        auto& vals = option_values.at(idx);
        if (vals.empty())
        {
            if (o.required)
                err(fmt::format("missing required option '--{}'", o.long_name));
            if (o.default_value)
                vals.push_back(o.default_value.value());
        }
    }

    // Check positional counts (skipped in raw-collect mode).
    if (not spec.raw_collect_positional
        && positionals.size()
        != spec.positional.size())
    {
        if (spec.positional.empty())
            err(fmt::format("unexpected positional argument: '{}'",
                            positionals.at(0)));
        else
            err(fmt::format("expected {} positional argument(s), got {}",
                            spec.positional.size(), positionals.size()));
    }

    // -- Build result --

    // Flags sub-map
    Map flags_map;
    for (std::size_t idx = 0; idx < spec.flags.size(); ++idx)
        flags_map.insert_or_assign(
            Value::create(String{spec.flags.at(idx).long_name}),
            Value::create(Bool{flag_values.at(idx)}));

    // Options sub-map
    Map options_map;
    for (const auto& [o, vals] : std::views::zip(spec.options, option_values))
    {
        auto key = Value::create(String{o.long_name});

        if (o.repeatable)
        {
            Array arr;
            for (const auto& v : vals)
                arr.push_back(Value::create(String{v}));
            options_map.insert_or_assign(std::move(key),
                                         Value::create(std::move(arr)));
        }
        else if (vals.empty())
        {
            options_map.insert_or_assign(std::move(key), Value::null());
        }
        else
        {
            options_map.insert_or_assign(std::move(key),
                                         Value::create(String{vals.at(0)}));
        }
    }

    // Positional array
    Array positional_arr;
    for (auto& p : positionals)
        positional_arr.push_back(Value::create(String{std::move(p)}));

    static const auto key_flags = "flags"_s;
    static const auto key_options = "options"_s;
    static const auto key_positional = "positional"_s;
    static const auto key_help = "help"_s;

    return Value::create(
        Value::trusted,
        Map{{key_flags, Value::create(Value::trusted, std::move(flags_map))},
            {key_help, Value::create(generate_help(spec, tool_name))},
            {key_options,
             Value::create(Value::trusted, std::move(options_map))},
            {key_positional, Value::create(std::move(positional_arr))}});
}

// -- Help generation --

String generate_help(const Cli_Spec& spec, const String& tool_name)
{
    if (spec.help_text)
        return spec.help_text.value();

    std::string out;

    // Usage line
    out += fmt::format("Usage: {}", tool_name);
    if (not spec.flags.empty() || not spec.options.empty())
        out += " [options]";
    if (spec.raw_collect_positional)
    {
        out += " [args...]";
    }
    else
    {
        for (const auto& p : spec.positional)
            out += fmt::format(" <{}>", p.name);
    }
    out += "\n";

    if (spec.description)
        out += fmt::format("\n{}\n", spec.description.value());

    // Options section
    if (not spec.flags.empty() || not spec.options.empty())
    {
        out += "\nOptions:\n";

        // Collect all entries for alignment
        struct Help_Entry
        {
            std::string left;
            std::string desc;
        };
        std::vector<Help_Entry> entries;

        for (const auto& f : spec.flags)
        {
            std::string left;
            if (f.short_name)
                left = fmt::format("  -{}, --{}", f.short_name.value(),
                                   f.long_name);
            else
                left = fmt::format("      --{}", f.long_name);
            entries.push_back({std::move(left), f.description.value_or("")});
        }

        for (const auto& o : spec.options)
        {
            std::string left;
            if (o.short_name)
                left = fmt::format("  -{}, --{} <value>", o.short_name.value(),
                                   o.long_name);
            else
                left = fmt::format("      --{} <value>", o.long_name);
            std::string desc = o.description.value_or("");
            if (o.required)
                desc += " (required)";
            else if (o.default_value)
                desc += fmt::format(" (default: {})", o.default_value.value());
            if (o.repeatable)
                desc += " (repeatable)";
            entries.push_back({std::move(left), std::move(desc)});
        }

        // Find max left width for alignment
        std::size_t max_left = 0;
        for (const auto& e : entries)
            max_left = std::max(max_left, e.left.size());

        for (const auto& e : entries)
        {
            if (e.desc.empty())
                out += fmt::format("{}\n", e.left);
            else
                out += fmt::format("{:<{}}  {}\n", e.left, max_left, e.desc);
        }
    }

    // Positional section
    if (not spec.positional.empty())
    {
        out += "\nPositional arguments:\n";

        std::size_t max_name = 0;
        for (const auto& p : spec.positional)
            max_name = std::max(max_name, p.name.size());

        for (const auto& p : spec.positional)
        {
            if (p.description)
                out += fmt::format("  {:<{}}  {}\n", p.name, max_name,
                                   p.description.value());
            else
                out += fmt::format("  {}\n", p.name);
        }
    }

    return out;
}

} // namespace

BUILTIN(parse)
{
    REQUIRE_ARGS("cli.parse", PARAM("args", TYPES(Array)),
                 PARAM("spec", TYPES(Map)));

    const auto& frost_args = GET(0, Array);
    const auto& spec_map = GET(1, Map);

    if (frost_args.empty())
        throw Frost_Recoverable_Error{"cli.parse: args must be non-empty"};

    for (const auto& [i, arg] : std::views::enumerate(frost_args))
        if (not arg->is<String>())
            throw Frost_Recoverable_Error{
                fmt::format("cli.parse: args[{}] must be a String, got {}", i,
                            arg->type_name())};

    auto spec = parse_spec(spec_map);
    validate_spec(spec);

    auto tool_name = spec.name.value_or(frost_args.at(0)->raw_get<String>());

    return parse_args(spec, frost_args, tool_name);
}

} // namespace cli

STDLIB_MODULE(cli, ENTRY(parse))

} // namespace frst
