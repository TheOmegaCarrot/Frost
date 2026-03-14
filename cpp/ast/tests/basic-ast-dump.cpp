#include <catch2/catch_test_macros.hpp>
#include <fmt/format.h>

#include <frost/ast.hpp>
#include <frost/symbol-table.hpp>

#include <ranges>
#include <sstream>
#include <utility>
#include <vector>

class String_Node : public frst::ast::Statement
{
  public:
    String_Node(std::string label)
        : Statement(no_range)
        , label_{label}
    {
    }

    std::optional<frst::Map> do_execute(
        [[maybe_unused]] frst::Execution_Context&) const override
    {
        return std::nullopt;
    }

  protected:
    std::string do_node_label() const override
    {
        return fmt::format("String_Node({})", label_);
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
        : Statement(no_range)
    {
        for (auto i = 1uz; i != num + 1; ++i)
            children_.emplace_back(
                std::make_unique<String_Node>(fmt::format("Child_{}", i)));
    }

    std::optional<frst::Map> do_execute(
        [[maybe_unused]] frst::Execution_Context&) const override
    {
        return std::nullopt;
    }

  protected:
    std::string do_node_label() const override
    {
        return fmt::format("Multi_String_Node({})", children_.size());
    }

    std::generator<Child_Info> children() const override
    {
        for (const auto& child : children_)
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
        : Statement(no_range)
        , first_{std::make_unique<String_Node>(first)}
        , second_{std::make_unique<String_Node>(second)}
        , third_{std::make_unique<String_Node>(third)}
    {
    }

    std::optional<frst::Map> do_execute(
        [[maybe_unused]] frst::Execution_Context&) const override
    {
        return std::nullopt;
    }

  protected:
    std::string do_node_label() const override
    {
        return fmt::format("Three_Children()");
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

class Tree_Node : public frst::ast::Statement
{
  public:
    struct Child
    {
        frst::ast::Statement::Ptr node;
        std::string label;
    };

    explicit Tree_Node(std::string label)
        : Statement(no_range)
        , label_{std::move(label)}
    {
    }

    std::optional<frst::Map> do_execute(
        [[maybe_unused]] frst::Execution_Context&) const override
    {
        return std::nullopt;
    }

    Tree_Node& add_child(frst::ast::Statement::Ptr child,
                         std::string label = {})
    {
        children_.push_back(Child{std::move(child), std::move(label)});
        return *this;
    }

  protected:
    std::string do_node_label() const override
    {
        return label_;
    }

    std::generator<Child_Info> children() const override
    {
        for (const auto& child : children_)
        {
            if (child.label.empty())
                co_yield make_child(child.node);
            else
                co_yield make_child(child.node, child.label);
        }
    }

  private:
    std::string label_;
    std::vector<Child> children_;
};

std::string dump_helper(const frst::ast::Statement& node)
{
    std::ostringstream buf;
    node.debug_dump_ast(buf);
    return std::move(buf).str();
}

TEST_CASE("Basic AST Dump")
{

    SECTION("String Node")
    {
        String_Node node{"Testing"};
        auto result = dump_helper(node);
        INFO(result);
        CHECK(result == "String_Node(Testing) [0:0-0:0]\n");
    }

    SECTION("Multi String Node")
    {
        Multi_String_Node node{3};
        auto result = dump_helper(node);
        INFO(result);
        CHECK(result ==
              R"(Multi_String_Node(3) [0:0-0:0]
├── String_Node(Child_1) [0:0-0:0]
├── String_Node(Child_2) [0:0-0:0]
└── String_Node(Child_3) [0:0-0:0]
)");
    }

    SECTION("Three Children")
    {
        Three_Children node{"One", "Two", "Three"};
        auto result = dump_helper(node);
        INFO(result);
        CHECK(result ==
              R"(Three_Children() [0:0-0:0]
├── First
│   └── String_Node(One) [0:0-0:0]
├── Second
│   └── String_Node(Two) [0:0-0:0]
└── Third
    └── String_Node(Three) [0:0-0:0]
)");
    }

    SECTION("Nested Unlabeled Children")
    {
        Tree_Node root{"Root"};
        auto child_one = std::make_unique<Tree_Node>("Child_One");
        child_one->add_child(std::make_unique<Tree_Node>("Leaf_A"))
            .add_child(std::make_unique<Tree_Node>("Leaf_B"));
        auto child_two = std::make_unique<Tree_Node>("Child_Two");
        child_two->add_child(std::make_unique<Tree_Node>("Leaf_C"));
        root.add_child(std::move(child_one));
        root.add_child(std::move(child_two));

        auto result = dump_helper(root);
        INFO(result);
        CHECK(result ==
              R"(Root [0:0-0:0]
├── Child_One [0:0-0:0]
│   ├── Leaf_A [0:0-0:0]
│   └── Leaf_B [0:0-0:0]
└── Child_Two [0:0-0:0]
    └── Leaf_C [0:0-0:0]
)");
    }

    SECTION("Labeled Child Inserts Wrapper")
    {
        Tree_Node root{"Mixed"};
        root.add_child(std::make_unique<Tree_Node>("Alpha"));
        root.add_child(std::make_unique<Tree_Node>("Bravo"), "Wrapped");
        root.add_child(std::make_unique<Tree_Node>("Charlie"));

        auto result = dump_helper(root);
        INFO(result);
        CHECK(result ==
              R"(Mixed [0:0-0:0]
├── Alpha [0:0-0:0]
├── Wrapped
│   └── Bravo [0:0-0:0]
└── Charlie [0:0-0:0]
)");
    }

    SECTION("Labeled Child With Subtree")
    {
        Tree_Node root{"Root"};
        auto branch = std::make_unique<Tree_Node>("Branch");
        branch->add_child(std::make_unique<Tree_Node>("Leaf_X"))
            .add_child(std::make_unique<Tree_Node>("Leaf_Y"));
        root.add_child(std::move(branch), "Group");
        root.add_child(std::make_unique<Tree_Node>("Tail"));

        auto result = dump_helper(root);
        INFO(result);
        CHECK(result ==
              R"(Root [0:0-0:0]
├── Group
│   └── Branch [0:0-0:0]
│       ├── Leaf_X [0:0-0:0]
│       └── Leaf_Y [0:0-0:0]
└── Tail [0:0-0:0]
)");
    }
}
