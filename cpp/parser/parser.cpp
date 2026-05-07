#include <frost/parser.hpp>
#include <frost/value.hpp>

#include <expected>

#include <fmt/core.h>

#include <lexy/action/parse.hpp>
#include <lexy/encoding.hpp>
#include <lexy/input/file.hpp>
#include <lexy/input/string_input.hpp>
#include <lexy_ext/report_error.hpp>

#include "grammar.hpp"

namespace frst
{
namespace
{

void assign_filepath(const std::vector<ast::Statement::Ptr>& program,
                     std::string_view filepath)
{
    auto filepath_ptr = std::make_shared<const std::string>(filepath);

    for (auto& statement : program)
    {
        for (auto* node : statement->walk())
            node->set_filepath(filepath_ptr);
    }
}

template <typename Input, typename Reporter>
std::expected<std::vector<ast::Statement::Ptr>, std::string> parse_impl(
    const Input& input, Reporter reporter)
{
    try
    {
        std::string err;

        grammar::reset_parse_state(input);
        auto result = lexy::parse<grammar::program>(
            input, reporter.to(std::back_inserter(err)));

        if (not result)
            return std::unexpected{err};

        return std::move(result).value();
    }
    catch (const Frost_User_Error& e)
    {
        return std::unexpected{e.what()};
    }
    catch (const Frost_Interpreter_Error& e)
    {
        return std::unexpected{fmt::format("Internal error: {}", e.what())};
    }
}
} // namespace

std::expected<std::vector<ast::Statement::Ptr>, std::string> parse_program(
    const std::string& program_text, std::string_view filepath)
{
    auto input = lexy::string_input<lexy::utf8_encoding>(program_text);

    auto result = parse_impl(
        input, lexy_ext::report_error.opts({lexy::visualize_fancy}));

    if (result)
        assign_filepath(result.value(), filepath);

    return result;
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

    auto result = parse_impl(
        file.buffer(), lexy_ext::report_error.path(path_str.c_str())
                           .opts({lexy::visualize_fancy}));

    if (result)
        assign_filepath(result.value(), path_str);

    return result;
}

std::expected<ast::Expression::Ptr, std::string> parse_data(
    const std::string& text)
{
    try
    {
        std::string err;

        auto input = lexy::string_input<lexy::utf8_encoding>(text);
        grammar::reset_parse_state(input);
        auto result = lexy::parse<grammar::data_expression>(
            input, lexy_ext::report_error.opts({lexy::visualize_fancy})
                       .to(std::back_inserter(err)));

        if (not result)
            return std::unexpected{err};

        return std::move(result).value();
    }
    catch (const Frost_User_Error& e)
    {
        return std::unexpected{e.what()};
    }
    catch (const Frost_Interpreter_Error& e)
    {
        return std::unexpected{fmt::format("Internal error: {}", e.what())};
    }
}

} // namespace frst
