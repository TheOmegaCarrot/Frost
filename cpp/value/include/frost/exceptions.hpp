#ifndef FROST_EXCEPTIONS_HPP
#define FROST_EXCEPTIONS_HPP

#include <stdexcept>
#include <string>

#include <boost/preprocessor/stringize.hpp>

namespace frst
{

struct Frost_Error : std::runtime_error
{
  protected:
    Frost_Error(const char* err)
        : std::runtime_error{err}
    {
    }

    Frost_Error(const std::string& err)
        : std::runtime_error{err}
    {
    }
};

//! Something has gone wrong inside the interpreter (a bug)
struct Frost_Interpreter_Error : Frost_Error
{
    Frost_Interpreter_Error(const char* err)
        : Frost_Error{err}
    {
    }

    Frost_Interpreter_Error(const std::string& err)
        : Frost_Error{err}
    {
    }
};

//! User code has encountered an error (it's the user's fault)
struct Frost_User_Error : Frost_Error
{
  protected:
    Frost_User_Error(const char* err)
        : Frost_Error{err}
    {
    }

    Frost_User_Error(const std::string& err)
        : Frost_Error{err}
    {
    }
};

//! User code can recover from this error (with try_call)
struct Frost_Recoverable_Error : Frost_User_Error
{
    Frost_Recoverable_Error(const char* err)
        : Frost_User_Error{err}
    {
    }

    Frost_Recoverable_Error(const std::string& err)
        : Frost_User_Error{err}
    {
    }
};

//! User code cannot recover from this error (fatal error due to user error)
struct Frost_Unrecoverable_Error : Frost_User_Error
{
    Frost_Unrecoverable_Error(const char* err)
        : Frost_User_Error{err}
    {
    }

    Frost_Unrecoverable_Error(const std::string& err)
        : Frost_User_Error{err}
    {
    }
};

#define THROW_UNREACHABLE                                                      \
    throw Frost_Interpreter_Error                                              \
    {                                                                          \
        "Hit point which should be unreachable at: " __FILE__                  \
        ":" BOOST_PP_STRINGIZE(__LINE__)                                       \
    }

} // namespace frst

#endif
