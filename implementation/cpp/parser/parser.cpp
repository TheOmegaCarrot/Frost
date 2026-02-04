#include <frost/parser.hpp>
#include <frost/value.hpp>

#include <expected>

#include <fmt/core.h>

#include <lexy/action/parse.hpp>
#include <lexy/input/file.hpp>
#include <lexy/input/string_input.hpp>
#include <lexy_ext/report_error.hpp>

#include "grammar.hpp"

namespace frst
{
namespace
{
template <typename Input, typename Reporter>
std::expected<std::vector<ast::Statement::Ptr>, std::string> parse_impl(
    const Input& input, Reporter reporter)
{
    std::string err;

    auto result = lexy::parse<grammar::program>(
        input, reporter.to(std::back_inserter(err)));

    if (!result)
        return std::unexpected{err};

    return std::move(result).value();
}
} // namespace

std::expected<std::vector<ast::Statement::Ptr>, std::string> parse_program(
    const std::string& program_text)
{
    auto input = lexy::string_input(program_text);

    try
    {
        return parse_impl(input,
                          lexy_ext::report_error.opts({lexy::visualize_fancy}));
    }
    catch (const Frost_User_Error& e)
    {
        return std::unexpected{e.what()};
    }
    catch (const Frost_Internal_Error& e)
    {
        return std::unexpected{fmt::format("Internal error: {}", e.what())};
    }
}

std::expected<std::vector<ast::Statement::Ptr>, std::string> parse_file(
    const std::filesystem::path& filename)
{
    auto path_str = filename.string();
    auto file = lexy::read_file<lexy::utf8_encoding>(path_str.c_str());
    if (!file)
    {
        return std::unexpected{
            fmt::format("Failed to read file '{}'", path_str)};
    }

    return parse_impl(file.buffer(),
                      lexy_ext::report_error.path(path_str.c_str())
                          .opts({lexy::visualize_fancy}));
}

} // namespace frst
