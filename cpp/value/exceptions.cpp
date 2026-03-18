#include <frost/backtrace.hpp>
#include <frost/exceptions.hpp>

namespace frst
{

static std::vector<std::string> snapshot_current()
{
    auto* bt = Backtrace_State::current();
    if (!bt)
        return {};
    return bt->capture_snapshot();
}

Frost_Error::Frost_Error(const char* err)
    : std::runtime_error{err}
    , backtrace_frames_{snapshot_current()}
{
}

Frost_Error::Frost_Error(const std::string& err)
    : std::runtime_error{err}
    , backtrace_frames_{snapshot_current()}
{
}

} // namespace frst
