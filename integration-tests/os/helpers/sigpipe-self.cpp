// Writes to stdout, flushes, then dies from SIGPIPE.
#include <cstdio>
#include <csignal>

int main()
{
    std::fputs("before sigpipe\n", stdout);
    std::fflush(stdout);
    std::raise(SIGPIPE);
}
