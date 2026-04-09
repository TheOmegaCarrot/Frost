#include <frost/builtins-common.hpp>

#include <frost/value.hpp>

#include <flat_map>
#include <flat_set>
#include <map>

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

void require_string_key(const Value_Ptr& k_val, std::string_view context)
{
    if (not k_val->is<String>())
        throw Frost_Recoverable_Error{
            fmt::format("cli.parse: {} keys must be Strings, got {}", context,
                        k_val->type_name())};
}

Flag_Spec parse_one_flag(const String& long_name, const Map& sub)
{
    Flag_Spec result;
    result.long_name = long_name;

    for (const auto& [k_val, v_val] : sub)
    {
        require_string_key(k_val, "flag spec");
        const auto& key = k_val->raw_get<String>();

        if (key == "short")
        {
            if (not v_val->is<String>())
                throw Frost_Recoverable_Error{fmt::format(
                    "cli.parse: flag '{}': short must be a String", long_name)};
            const auto& s = v_val->raw_get<String>();
            if (s.size() != 1)
                throw Frost_Recoverable_Error{fmt::format(
                    "cli.parse: flag '{}': short must be a single character",
                    long_name)};
            result.short_name = s.at(0);
        }
        else if (key == "description")
        {
            if (not v_val->is<String>())
                throw Frost_Recoverable_Error{fmt::format(
                    "cli.parse: flag '{}': description must be a String",
                    long_name)};
            result.description = v_val->raw_get<String>();
        }
        else
        {
            throw Frost_Recoverable_Error{fmt::format(
                "cli.parse: flag '{}': unknown key '{}'", long_name, key)};
        }
    }

    return result;
}

std::vector<Flag_Spec> parse_flags(const Map& flags_map)
{
    std::vector<Flag_Spec> result;

    for (const auto& [k_val, v_val] : flags_map)
    {
        require_string_key(k_val, "flags");
        const auto& long_name = k_val->raw_get<String>();

        if (not v_val->is<Map>())
            throw Frost_Recoverable_Error{fmt::format(
                "cli.parse: flag '{}' spec must be a Map", long_name)};

        result.push_back(parse_one_flag(long_name, v_val->raw_get<Map>()));
    }

    return result;
}

Option_Spec parse_one_option(const String& long_name, const Map& sub)
{
    Option_Spec result;
    result.long_name = long_name;

    for (const auto& [k_val, v_val] : sub)
    {
        require_string_key(k_val, "option spec");
        const auto& key = k_val->raw_get<String>();

        if (key == "short")
        {
            if (not v_val->is<String>())
                throw Frost_Recoverable_Error{fmt::format(
                    "cli.parse: option '{}': short must be a String",
                    long_name)};
            const auto& s = v_val->raw_get<String>();
            if (s.size() != 1)
                throw Frost_Recoverable_Error{fmt::format(
                    "cli.parse: option '{}': short must be a single character",
                    long_name)};
            result.short_name = s.at(0);
        }
        else if (key == "required")
        {
            if (not v_val->is<Bool>())
                throw Frost_Recoverable_Error{fmt::format(
                    "cli.parse: option '{}': required must be a Bool",
                    long_name)};
            result.required = v_val->raw_get<Bool>();
        }
        else if (key == "default")
        {
            if (not v_val->is<String>())
                throw Frost_Recoverable_Error{fmt::format(
                    "cli.parse: option '{}': default must be a String",
                    long_name)};
            result.default_value = v_val->raw_get<String>();
        }
        else if (key == "repeatable")
        {
            if (not v_val->is<Bool>())
                throw Frost_Recoverable_Error{fmt::format(
                    "cli.parse: option '{}': repeatable must be a Bool",
                    long_name)};
            result.repeatable = v_val->raw_get<Bool>();
        }
        else if (key == "description")
        {
            if (not v_val->is<String>())
                throw Frost_Recoverable_Error{fmt::format(
                    "cli.parse: option '{}': description must be a String",
                    long_name)};
            result.description = v_val->raw_get<String>();
        }
        else
        {
            throw Frost_Recoverable_Error{fmt::format(
                "cli.parse: option '{}': unknown key '{}'", long_name, key)};
        }
    }

    return result;
}

std::vector<Option_Spec> parse_options(const Map& options_map)
{
    std::vector<Option_Spec> result;

    for (const auto& [k_val, v_val] : options_map)
    {
        require_string_key(k_val, "options");
        const auto& long_name = k_val->raw_get<String>();

        if (not v_val->is<Map>())
            throw Frost_Recoverable_Error{fmt::format(
                "cli.parse: option '{}' spec must be a Map", long_name)};

        result.push_back(parse_one_option(long_name, v_val->raw_get<Map>()));
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
        if (not entry->is<Map>())
            throw Frost_Recoverable_Error{
                fmt::format("cli.parse: positional[{}] must be a Map", i)};

        Positional_Spec spec;
        bool has_name = false;

        for (const auto& [k_val, v_val] : entry->raw_get<Map>())
        {
            require_string_key(k_val, "positional spec");
            const auto& key = k_val->raw_get<String>();

            if (key == "name")
            {
                if (not v_val->is<String>())
                    throw Frost_Recoverable_Error{fmt::format(
                        "cli.parse: positional[{}]: name must be a String", i)};
                spec.name = v_val->raw_get<String>();
                has_name = true;
            }
            else if (key == "description")
            {
                if (not v_val->is<String>())
                    throw Frost_Recoverable_Error{fmt::format(
                        "cli.parse: positional[{}]: description must be a "
                        "String",
                        i)};
                spec.description = v_val->raw_get<String>();
            }
            else
            {
                throw Frost_Recoverable_Error{fmt::format(
                    "cli.parse: positional[{}]: unknown key '{}'", i, key)};
            }
        }

        if (not has_name)
            throw Frost_Recoverable_Error{
                fmt::format("cli.parse: positional[{}]: name is required", i)};

        result.push_back(std::move(spec));
    }

    return result;
}

Cli_Spec parse_spec(const Map& spec_map)
{
    Cli_Spec spec;

    for (const auto& [k_val, v_val] : spec_map)
    {
        require_string_key(k_val, "spec");
        const auto& key = k_val->raw_get<String>();

        if (key == "name")
        {
            if (not v_val->is<String>())
                throw Frost_Recoverable_Error{
                    "cli.parse: name must be a String"};
            spec.name = v_val->raw_get<String>();
        }
        else if (key == "description")
        {
            if (not v_val->is<String>())
                throw Frost_Recoverable_Error{
                    "cli.parse: description must be a String"};
            spec.description = v_val->raw_get<String>();
        }
        else if (key == "help")
        {
            if (not v_val->is<String>())
                throw Frost_Recoverable_Error{
                    "cli.parse: help must be a String"};
            spec.help_text = v_val->raw_get<String>();
        }
        else if (key == "flags")
        {
            if (not v_val->is<Map>())
                throw Frost_Recoverable_Error{"cli.parse: flags must be a Map"};
            spec.flags = parse_flags(v_val->raw_get<Map>());
        }
        else if (key == "options")
        {
            if (not v_val->is<Map>())
                throw Frost_Recoverable_Error{
                    "cli.parse: options must be a Map"};
            spec.options = parse_options(v_val->raw_get<Map>());
        }
        else if (key == "positional")
        {
            if (v_val->is<Bool>() and v_val->raw_get<Bool>())
                spec.raw_collect_positional = true;
            else
                spec.positional = parse_positional(v_val);
        }
        else
        {
            throw Frost_Recoverable_Error{
                fmt::format("cli.parse: unknown spec key '{}'", key)};
        }
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
    // State is pre-populated with every declared flag/option so later
    // lookups are safe.
    std::map<String, bool> flag_values;
    for (const auto& f : spec.flags)
        flag_values.emplace(f.long_name, false);

    std::map<String, std::vector<String>> option_values;
    for (const auto& o : spec.options)
        option_values.emplace(o.long_name, std::vector<String>{});

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
                flag_values.at(name) = true;
                continue;
            }

            auto oit = spec.long_to_option.find(name);
            if (oit != spec.long_to_option.end())
            {
                if (i + 1 >= args.size())
                    err(fmt::format("option '--{}' requires a value", name));
                ++i;
                const auto& val = args.at(i)->raw_get<String>();
                auto& vals = option_values.at(name);
                if (not vals.empty()
                    and not spec.options.at(oit->second).repeatable)
                    err(fmt::format("option '--{}' cannot be repeated", name));
                vals.push_back(val);
                continue;
            }

            err(fmt::format("unknown option '--{}'", name));
        }

        // Short form: -abc
        // Index loop because a value-taking option must be the last char
        // in the bundle, and we need the position to enforce that.
        for (std::size_t j = 1; j < arg.size(); ++j)
        {
            char c = arg.at(j);

            auto fit = spec.short_to_flag.find(c);
            if (fit != spec.short_to_flag.end())
            {
                flag_values.at(spec.flags.at(fit->second).long_name) = true;
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
                const auto& opt_name = spec.options.at(oit->second).long_name;
                if (i + 1 >= args.size())
                    err(fmt::format("option '-{}' requires a value", c));
                ++i;
                const auto& val = args.at(i)->raw_get<String>();
                auto& vals = option_values.at(opt_name);
                if (not vals.empty()
                    and not spec.options.at(oit->second).repeatable)
                    err(fmt::format("option '-{}' cannot be repeated", c));
                vals.push_back(val);
                break; // value-taking option ends the bundle
            }

            err(fmt::format("unknown option '-{}'", c));
        }
    }

    // Post-walk: check required options and apply defaults
    for (const auto& o : spec.options)
    {
        auto& vals = option_values.at(o.long_name);
        if (vals.empty())
        {
            if (o.required)
                err(fmt::format("missing required option '--{}'", o.long_name));
            if (o.default_value)
                vals.push_back(o.default_value.value());
        }
    }

    // Check positional counts
    if (spec.raw_collect_positional)
    {
        // Raw collect mode -- no validation
    }
    else if (positionals.size() != spec.positional.size())
    {
        if (spec.positional.empty() and not positionals.empty())
            err(fmt::format("unexpected positional argument: '{}'",
                            positionals.at(0)));
        else
            err(fmt::format("expected {} positional argument(s), got {}",
                            spec.positional.size(), positionals.size()));
    }

    // -- Build result --

    // Flags sub-map
    Map flags_map;
    for (const auto& [name, was_set] : flag_values)
        flags_map.insert_or_assign(Value::create(String{name}),
                                   Value::create(Bool{was_set}));

    // Options sub-map
    Map options_map;
    for (const auto& o : spec.options)
    {
        const auto& vals = option_values.at(o.long_name);
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
    if (not spec.flags.empty() or not spec.options.empty())
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
    if (not spec.flags.empty() or not spec.options.empty())
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
