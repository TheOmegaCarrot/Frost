#include <frost/builtins-common.hpp>
#include <frost/streams.hpp>

#include <frost/builtin.hpp>
#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <iostream>

namespace frst
{

using namespace streams_detail;

STRINGS(read_line, read_one, read, read_rest, write, writeln);

namespace
{

Value_Ptr make_stdin()
{
    // A shared ptr to std::cin with a no-op delete...
    // I'm not proud of this...
    auto hacky_stdin_ptr = std::make_shared<Locked_Stream<std::istream>>(
        std::shared_ptr<std::istream>(&std::cin, [](auto&&...) {
        }));

    return Value::create(Value::trusted,
                         Map{
                             {strings.read_line, read_line(hacky_stdin_ptr)},
                             {strings.read_one, read_one(hacky_stdin_ptr)},
                             {strings.read, read_rest(hacky_stdin_ptr)},
                             {strings.read_rest, read_rest(hacky_stdin_ptr)},
                         });
}

Value_Ptr make_stderr()
{
    auto hacky_stderr_ptr = std::make_shared<Locked_Stream<std::ostream>>(
        std::shared_ptr<std::ostream>(&std::clog, [](auto&&...) {
        }));

    return Value::create(Value::trusted,
                         Map{
                             {strings.write, write(hacky_stderr_ptr)},
                             {strings.writeln, writeln(hacky_stderr_ptr)},
                         });
}

Value_Ptr make_stdout()
{
    auto hacky_stdout_ptr = std::make_shared<Locked_Stream<std::ostream>>(
        std::shared_ptr<std::ostream>(&std::cout, [](auto&&...) {
        }));

    return Value::create(Value::trusted,
                         Map{
                             {strings.write, write(hacky_stdout_ptr)},
                             {strings.writeln, writeln(hacky_stdout_ptr)},
                         });
}

} // namespace

void inject_streams(Symbol_Table& table)
{
    table.define("stdin", make_stdin());
    table.define("stderr", make_stderr());
    table.define("stdout", make_stdout());
}

} // namespace frst
