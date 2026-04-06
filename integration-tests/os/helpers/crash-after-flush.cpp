// Writes to stdout, flushes, then crashes with SIGSEGV.
#include <cstdio>
#include <csignal>

int main()
{
    std::fputs("before crash\n", stdout);
    std::fflush(stdout);
    std::raise(SIGSEGV);
}
