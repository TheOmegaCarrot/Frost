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

std::expected<std::vector<ast::Statement::Ptr>, std::string> parse_program(
    const std::string& program_text)
{
    auto input = lexy::string_input(program_text);

    std::string err;

    auto result = lexy::parse<grammar::program>(
        input, lexy_ext::report_error.opts({lexy::visualize_fancy})
                   .to(std::back_inserter(err)));

    if (!result)
        return std::unexpected{err};

    return std::move(result).value();
}

std::expected<std::vector<ast::Statement::Ptr>, std::string> parse_file(
    const std::filesystem::path& filename)
{
    auto path_str = filename.string();
    auto file = lexy::read_file<lexy::utf8_encoding>(path_str.c_str());
    if (!file)
    {
        throw Frost_User_Error{
            fmt::format("Failed to read file '{}'", path_str)};
    }

    std::string err;

    auto result = lexy::parse<grammar::program>(
        file.buffer(), lexy_ext::report_error.path(path_str.c_str())
                           .opts({lexy::visualize_fancy})
                           .to(std::back_inserter(err)));

    if (!result)
        return std::unexpected{err};

    return std::move(result).value();
}

} // namespace frst
