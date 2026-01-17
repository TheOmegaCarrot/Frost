#include <frost/parser.hpp>

#include "grammar.hpp"

using namespace frst;

std::vector<ast::Statement::Ptr> parse_program(const std::string& program_text)
{
    (void)program_text;
    return {};
}

std::vector<ast::Statement::Ptr> parse_file(
    const std::filesystem::path& filename)
{
    (void)filename;
    return {};
}
