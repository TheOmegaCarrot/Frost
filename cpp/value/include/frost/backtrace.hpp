#ifndef FROST_BACKTRACE_HPP
#define FROST_BACKTRACE_HPP

#include <string>
#include <variant>
#include <vector>

namespace frst
{

namespace ast
{
class Statement;
}

struct AST_Frame
{
    const ast::Statement* node;
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
    std::string operation;
    std::string function_name;
};

using Backtrace_Frame =
    std::variant<AST_Frame, Call_Frame, Import_Frame, Iterative_Frame>;

class Backtrace_State
{
  public:
    Backtrace_State() { frames_.reserve(256); }

    void push(Backtrace_Frame frame) { frames_.push_back(std::move(frame)); }
    void pop() { frames_.pop_back(); }

    // Format the current live stack into owned strings.
    // Defined in ast/backtrace.cpp (needs Statement complete type).
    std::vector<std::string> capture_snapshot();

    static Backtrace_State* current() { return current_state_; }
    static void set_current(Backtrace_State* s) { current_state_ = s; }

  private:
    std::vector<Backtrace_Frame> frames_;
    static inline thread_local Backtrace_State* current_state_ = nullptr;
};

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
            state_->pop();
    }

  private:
    Backtrace_State* state_;
};

} // namespace frst

#endif
