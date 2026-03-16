#ifndef FROST_BACKTRACE_HPP
#define FROST_BACKTRACE_HPP

#include <cstddef>
#include <exception>
#include <memory>
#include <string>
#include <variant>
#include <vector>

namespace frst
{

// Forward declaration
namespace ast
{
class Statement;
}

// ============================================================
// Live frame types — stored on the mutable stack during execution
// ============================================================

struct AST_Frame
{
    const ast::Statement* node; // non-owning, valid during execution
};

struct Call_Frame
{
    std::string function_name;
};

struct Import_Frame
{
    std::string module_spec;
};

struct Iterative_Frame
{
    std::string operation; // "Map", "Filter", "Reduce", "Foreach"
    std::string function_name;
};

using Backtrace_Frame =
    std::variant<AST_Frame, Call_Frame, Import_Frame, Iterative_Frame>;

// ============================================================
// Snapshot frame types — AST pointers resolved to strings
// ============================================================

struct Resolved_AST_Frame
{
    std::string node_label;
    std::string source_range;
};

using Snapshot_Frame =
    std::variant<Resolved_AST_Frame, Call_Frame, Import_Frame, Iterative_Frame>;

// ============================================================
// Backtrace — the resolved output, produced on error
// ============================================================

class Backtrace
{
  public:
    explicit Backtrace(std::vector<Snapshot_Frame> frames)
        : frames_{std::move(frames)}
    {
    }

    const std::vector<Snapshot_Frame>& frames() const
    {
        return frames_;
    }

  private:
    std::vector<Snapshot_Frame> frames_;
};

// ============================================================
// Backtrace_State — the live mutable state during execution
// ============================================================

class Backtrace_State
{
  public:
    Backtrace_State()
    {
        frames_.reserve(256);
    }

    void push(Backtrace_Frame frame)
    {
        snapshot_consumed_ = false;
        frames_.push_back(std::move(frame));
    }

    void pop()
    {
        frames_.pop_back();
    }

    // Called during stack unwinding (from Frame_Guard destructor)
    // and from the evaluate() catch block.
    // Captures the full frame stack the first time only.
    void snapshot_if_needed()
    {
        if (!snapshot_consumed_ && snapshot_.empty() && !frames_.empty())
            snapshot_ = frames_;
    }

    // Move the raw snapshot out and clear it for future errors.
    // The caller must resolve AST_Frame pointers before they go stale.
    std::vector<Backtrace_Frame> take_raw_snapshot()
    {
        snapshot_consumed_ = true;
        auto result = std::move(snapshot_);
        snapshot_.clear();
        return result;
    }

  private:
    std::vector<Backtrace_Frame> frames_;
    std::vector<Backtrace_Frame> snapshot_;
    bool snapshot_consumed_ = false;
};

// ============================================================
// Frame_Guard — RAII push/pop with snapshot on unwind
// Accepts a nullable Backtrace_State*; no-op when null.
// ============================================================

class Frame_Guard
{
  public:
    Frame_Guard(Backtrace_State* state, Backtrace_Frame frame)
        : state_{state}
    {
        if (state_)
            state_->push(std::move(frame));
    }

    Frame_Guard(const Frame_Guard&) = delete;
    Frame_Guard& operator=(const Frame_Guard&) = delete;
    Frame_Guard(Frame_Guard&&) = delete;
    Frame_Guard& operator=(Frame_Guard&&) = delete;

    ~Frame_Guard()
    {
        if (state_)
        {
            if (std::uncaught_exceptions() > 0)
                state_->snapshot_if_needed();
            state_->pop();
        }
    }

  private:
    Backtrace_State* state_;
};

} // namespace frst

#endif
