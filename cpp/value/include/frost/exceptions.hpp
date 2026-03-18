#ifndef FROST_EXCEPTIONS_HPP
#define FROST_EXCEPTIONS_HPP

#include <frost/backtrace.hpp>

#include <stdexcept>
#include <string>
#include <vector>

#include <boost/preprocessor/stringize.hpp>

namespace frst
{

struct Frost_Error : std::runtime_error
{
    std::vector<std::string> take_backtrace()
    {
        return std::move(backtrace_frames_);
    }

  protected:
    Frost_Error(const char* err)
        : std::runtime_error{err}
        , backtrace_frames_{snapshot_current()}
    {
    }

    Frost_Error(const std::string& err)
        : std::runtime_error{err}
        , backtrace_frames_{snapshot_current()}
    {
    }

  private:
    static std::vector<std::string> snapshot_current()
    {
        auto* bt = Backtrace_State::current();
        if (!bt)
            return {};
        return bt->capture_snapshot();
    }

    std::vector<std::string> backtrace_frames_;
};

struct Frost_Interpreter_Error : Frost_Error
{
    Frost_Interpreter_Error(const char* err)        : Frost_Error{err} {}
    Frost_Interpreter_Error(const std::string& err) : Frost_Error{err} {}
};

struct Frost_User_Error : Frost_Error
{
  protected:
    Frost_User_Error(const char* err)        : Frost_Error{err} {}
    Frost_User_Error(const std::string& err) : Frost_Error{err} {}
};

struct Frost_Recoverable_Error : Frost_User_Error
{
    Frost_Recoverable_Error(const char* err)        : Frost_User_Error{err} {}
    Frost_Recoverable_Error(const std::string& err) : Frost_User_Error{err} {}
};

struct Frost_Unrecoverable_Error : Frost_User_Error
{
    Frost_Unrecoverable_Error(const char* err)        : Frost_User_Error{err} {}
    Frost_Unrecoverable_Error(const std::string& err) : Frost_User_Error{err} {}
};

#define THROW_UNREACHABLE                                                      \
    throw Frost_Interpreter_Error                                              \
    {                                                                          \
        "Hit point which should be unreachable at: " __FILE__                  \
        ":" BOOST_PP_STRINGIZE(__LINE__)                                       \
    }

} // namespace frst

#endif
