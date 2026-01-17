#include <frost/parser.hpp>
#include <frost/value.hpp>

#include <fmt/core.h>

#include <lexy/action/parse.hpp>
#include <lexy/input/file.hpp>
#include <lexy/input/string_input.hpp>
#include <lexy_ext/report_error.hpp>

#include "grammar.hpp"

using namespace frst;

std::optional<std::vector<ast::Statement::Ptr>> parse_program(
    const std::string& program_text)
{
    auto input = lexy::string_input(program_text);
    auto result = lexy::parse<grammar::program>(input, lexy_ext::report_error);

    if (!result)
        return std::nullopt;

    return std::move(result).value();
}

std::optional<std::vector<ast::Statement::Ptr>> parse_file(
    const std::filesystem::path& filename)
{
    auto path_str = filename.string();
    auto file = lexy::read_file<lexy::utf8_encoding>(path_str.c_str());
    if (!file)
    {
        throw Frost_User_Error{
            fmt::format("Failed to read file '{}'", path_str)};
    }

    auto result = lexy::parse<grammar::program>(
        file.buffer(), lexy_ext::report_error.path(path_str.c_str()));

    if (!result)
        return std::nullopt;

    return std::move(result).value();
}
