#ifndef FROST_PARSER_HPP
#define FROST_PARSER_HPP

#include <frost/ast.hpp>

#include <filesystem>
#include <vector>

namespace frst
{
[[nodiscard]] std::vector<ast::Statement::Ptr> parse_program(
    const std::string& program_text);

[[nodiscard]] std::vector<ast::Statement::Ptr> parse_file(
    const std::filesystem::path& filename);
} // namespace frst

#endif
