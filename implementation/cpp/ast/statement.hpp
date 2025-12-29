#ifndef FROST_STATEMENT_HPP
#define FROST_STATEMENT_HPP

#include <memory>
#include <optional>
#include <ostream>
#include <string>
#include <string_view>

namespace frst::ast {

//! @brief Common base class of all AST nodes / statements
class Statement {
   public:
    using Ptr = std::unique_ptr<Statement>;

    Statement() = default;
    Statement(const Statement&) = delete;
    Statement(Statement&&) = delete;
    Statement& operator=(const Statement&) = delete;
    Statement& operator=(Statement&&) = delete;
    virtual ~Statement() = default;

    //! @brief Print AST of this node and all descendents
    void debug_dump_ast(std::ostream& out) const;

   protected:
    struct Child_Info {
        const Statement* node = nullptr;
        std::string_view label;
    };

    virtual std::string node_label() const = 0;

    //! @brief Index into list of children. Out-of-bounds
    virtual std::optional<Child_Info> child_at(std::size_t index) const {
        return std::nullopt;
    }

   private:
    void debug_dump_ast_impl(std::ostream& out, std::string_view prefix,
                             bool is_last, bool is_root) const;
};

}  // namespace frst::ast

#endif
