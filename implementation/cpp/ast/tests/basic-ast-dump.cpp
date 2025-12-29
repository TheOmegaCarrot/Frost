#include <catch2/catch_test_macros.hpp>
#include <frost/ast/ast.hpp>
#include <ranges>
#include <sstream>

#include <print>

class String_Node : public frst::ast::Statement
{
  public:
    String_Node(std::string label) : label_{label}
    {
    }

  protected:
    std::string node_label() const override
    {
        return std::format("String_Node({})", label_);
    }

    std::generator<Child_Info> children() const override
    {
        co_return;
    }

  private:
    std::string label_;
};

class Multi_String_Node : public frst::ast::Statement
{
  public:
    Multi_String_Node(std::size_t num)
    {
        for (auto i = 1uz; i != num + 1; ++i)
            children_.emplace_back(
                std::make_unique<String_Node>(std::format("Child_{}", i)));
    }

  protected:
    std::string node_label() const override
    {
        return std::format("Multi_String_Node({})", children_.size());
    }

    std::generator<Child_Info> children() const override
    {
        for (const auto& [i, child] : std::views::enumerate(children_))
        {
            co_yield make_child(child);
        }
    }

  private:
    std::vector<std::unique_ptr<Statement>> children_;
};

class Three_Children : public frst::ast::Statement
{
  public:
    Three_Children(std::string first, std::string second, std::string third)
        : first_{std::make_unique<String_Node>(first)},
          second_{std::make_unique<String_Node>(second)},
          third_{std::make_unique<String_Node>(third)}
    {
    }

  protected:
    std::string node_label() const override
    {
        return std::format("Three_Children()");
    }

    std::generator<Child_Info> children() const override
    {
        co_yield make_child(first_, "First");
        co_yield make_child(second_, "Second");
        co_yield make_child(third_, "Third");
    }

  private:
    frst::ast::Statement::Ptr first_;
    frst::ast::Statement::Ptr second_;
    frst::ast::Statement::Ptr third_;
};

std::string dump_helper(const frst::ast::Statement& node)
{
    std::ostringstream buf;
    node.debug_dump_ast(buf);
    return std::move(buf).str();
}

TEST_CASE("String Node")
{
    String_Node node{"Testing"};
    auto result = dump_helper(node);
    INFO(result);
    CHECK(result == "String_Node(Testing)\n");
}

TEST_CASE("Multi String Node")
{
    Multi_String_Node node{3};
    auto result = dump_helper(node);
    INFO(result);
    CHECK(result ==
          R"(Multi_String_Node(3)
├── String_Node(Child_1)
├── String_Node(Child_2)
└── String_Node(Child_3)
)");
}

TEST_CASE("Three Children")
{
    Three_Children node{"One", "Two", "Three"};
    auto result = dump_helper(node);
    INFO(result);
    CHECK(result ==
          R"(Three_Children()
├── First
│   └── String_Node(One)
├── Second
│   └── String_Node(Two)
└── Third
    └── String_Node(Three)
)");
}
