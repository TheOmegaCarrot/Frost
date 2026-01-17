#ifndef FROST_PARSER_HPP
#define FROST_PARSER_HPP

#include <frost/ast.hpp>

#include <expected>
#include <filesystem>
#include <vector>

namespace frst
{
[[nodiscard]] std::expected<std::vector<ast::Statement::Ptr>, std::string>
parse_program(const std::string& program_text);

[[nodiscard]] std::expected<std::vector<ast::Statement::Ptr>, std::string>
parse_file(const std::filesystem::path& filename);
} // namespace frst

#endif
