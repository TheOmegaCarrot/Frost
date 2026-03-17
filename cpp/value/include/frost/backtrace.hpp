#ifndef FROST_BACKTRACE_HPP
#define FROST_BACKTRACE_HPP

#include <exception>
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
// Backtrace_State — shared mutable frame stack
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
        frames_.push_back(std::move(frame));
    }

    void pop()
    {
        frames_.pop_back();
    }

    // Resolve the entire live stack into formatted strings.
    // Called by the first Frame_Guard destructor to detect unwinding.
    void snapshot_if_needed()
    {
        if (snapshot_.empty() && !frames_.empty())
            do_snapshot(); // defined in ast/backtrace.cpp
    }

    // Move the resolved snapshot out. Returns empty if no snapshot.
    std::vector<std::string> take_snapshot()
    {
        auto result = std::move(snapshot_);
        snapshot_.clear();
        return result;
    }

    static Backtrace_State* current() { return current_state_; }
    static void set_current(Backtrace_State* s) { current_state_ = s; }

  private:
    // Defined in ast/backtrace.cpp (needs Statement complete type).
    void do_snapshot();

    std::vector<Backtrace_Frame> frames_;
    std::vector<std::string> snapshot_;
    static inline thread_local Backtrace_State* current_state_ = nullptr;
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
