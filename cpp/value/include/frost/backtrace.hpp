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

// ============================================================
// Frame types
// ============================================================

struct AST_Frame
{
    std::string node_label;
    std::string source_range;
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
// Backtrace — the formatted output, produced on error
// ============================================================

class Backtrace
{
  public:
    explicit Backtrace(std::vector<Backtrace_Frame> frames)
        : frames_{std::move(frames)}
    {
    }

    const std::vector<Backtrace_Frame>& frames() const
    {
        return frames_;
    }

  private:
    std::vector<Backtrace_Frame> frames_;
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
        frames_.push_back(std::move(frame));
    }

    void pop()
    {
        frames_.pop_back();
    }

    // Called during stack unwinding (from Frame_Guard destructor).
    // Captures the full frame stack the first time only.
    void snapshot_if_needed()
    {
        if (snapshot_.empty() && !frames_.empty())
            snapshot_ = frames_;
    }

    // Called by catch sites (top-level or try_call).
    // Moves the snapshot out and clears it for future errors.
    std::unique_ptr<Backtrace> take_snapshot()
    {
        if (snapshot_.empty())
            return nullptr;

        auto bt = std::make_unique<Backtrace>(std::move(snapshot_));
        snapshot_.clear();
        return bt;
    }

    const std::vector<Backtrace_Frame>& frames() const
    {
        return frames_;
    }

  private:
    std::vector<Backtrace_Frame> frames_;
    std::vector<Backtrace_Frame> snapshot_;
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
