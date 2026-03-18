#ifndef FROST_BACKTRACE_HPP
#define FROST_BACKTRACE_HPP

#include <algorithm>
#include <optional>
#include <ranges>
#include <string>
#include <vector>

#include <fmt/format.h>

namespace frst
{

class Backtrace_State
{
  public:
    Backtrace_State() { frames_.reserve(256); }

    void push(std::string frame) { frames_.push_back(std::move(frame)); }
    void pop() { frames_.pop_back(); }

    std::vector<std::string> capture_snapshot() const
    {
        return frames_
               | std::views::reverse
               | std::ranges::to<std::vector<std::string>>();
    }

    static Backtrace_State* current() { return current_state_; }
    static void set_current(Backtrace_State* s) { current_state_ = s; }

  private:
    std::vector<std::string> frames_;
    static inline thread_local Backtrace_State* current_state_ = nullptr;
};

class Frame_Guard
{
  public:
    Frame_Guard(Backtrace_State* state, std::string frame)
        : state_{state}
    {
        if (state_)
            state_->push(std::move(frame));
    }

    Frame_Guard(const Frame_Guard&) = delete;
    Frame_Guard& operator=(const Frame_Guard&) = delete;
    Frame_Guard& operator=(Frame_Guard&&) = delete;

    Frame_Guard(Frame_Guard&& other) noexcept
        : state_{std::exchange(other.state_, nullptr)}
    {
    }

    ~Frame_Guard()
    {
        if (state_)
            state_->pop();
    }

  private:
    Backtrace_State* state_;
};

template <typename... Ts>
std::optional<Frame_Guard> make_frame_guard(fmt::format_string<Ts...> fmt,
                                            Ts&&... args)
{
    auto* bt = Backtrace_State::current();
    if (!bt)
        return std::nullopt;
    return Frame_Guard{bt, fmt::format(fmt, std::forward<Ts>(args)...)};
}

} // namespace frst

#endif
