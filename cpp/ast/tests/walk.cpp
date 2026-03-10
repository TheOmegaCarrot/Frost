#include <catch2/catch_test_macros.hpp>

#include <frost/ast.hpp>
#include <frost/symbol-table.hpp>

#include <string>
#include <vector>

using namespace frst;

namespace
{

// Minimal mock node with no children.
class Leaf_Node : public ast::Statement
{
  public:
    explicit Leaf_Node(std::string id) : id_{std::move(id)} {}

    std::optional<Map> execute(Symbol_Table&) const override
    {
        return std::nullopt;
    }

    std::string node_label() const override { return id_; }

  private:
    std::string id_;
};

// Mock node that can hold an arbitrary list of children.
class Parent_Node : public ast::Statement
{
  public:
    explicit Parent_Node(std::string id) : id_{std::move(id)} {}

    std::optional<Map> execute(Symbol_Table&) const override
    {
        return std::nullopt;
    }

    std::string node_label() const override { return id_; }

    void add(Ptr child) { children_.push_back(std::move(child)); }

  protected:
    std::generator<Child_Info> children() const override
    {
        for (const auto& child : children_)
            co_yield make_child(child);
    }

  private:
    std::string id_;
    std::vector<Ptr> children_;
};

std::vector<std::string> walk_labels(const ast::Statement& node)
{
    std::vector<std::string> result;
    for (const auto* n : node.walk())
        result.push_back(n->node_label());
    return result;
}

} // namespace

TEST_CASE("Statement::walk()")
{
    SECTION("Leaf node yields only itself")
    {
        Leaf_Node leaf{"A"};
        auto labels = walk_labels(leaf);
        REQUIRE(labels.size() == 1);
        CHECK(labels[0] == "A");
    }

    SECTION("Node with children yields root first, then children in order")
    {
        Parent_Node root{"Root"};
        root.add(std::make_unique<Leaf_Node>("B"));
        root.add(std::make_unique<Leaf_Node>("C"));

        auto labels = walk_labels(root);
        REQUIRE(labels.size() == 3);
        CHECK(labels[0] == "Root");
        CHECK(labels[1] == "B");
        CHECK(labels[2] == "C");
    }

    SECTION("Depth-first: visits subtree before next sibling")
    {
        // Tree:  Root
        //        ├── Child1
        //        │   └── GrandChild
        //        └── Child2
        Parent_Node root{"Root"};
        auto child1 = std::make_unique<Parent_Node>("Child1");
        child1->add(std::make_unique<Leaf_Node>("GrandChild"));
        root.add(std::move(child1));
        root.add(std::make_unique<Leaf_Node>("Child2"));

        auto labels = walk_labels(root);
        REQUIRE(labels.size() == 4);
        CHECK(labels[0] == "Root");
        CHECK(labels[1] == "Child1");
        CHECK(labels[2] == "GrandChild");
        CHECK(labels[3] == "Child2");
    }

    SECTION("Walk yields pointers to the actual nodes")
    {
        Parent_Node root{"Root"};
        auto child = std::make_unique<Leaf_Node>("Child");
        const auto* child_ptr = child.get();
        root.add(std::move(child));

        std::vector<const ast::Statement*> nodes;
        for (const auto* n : root.walk())
            nodes.push_back(n);

        REQUIRE(nodes.size() == 2);
        CHECK(nodes[0] == &root);
        CHECK(nodes[1] == child_ptr);
    }
}
