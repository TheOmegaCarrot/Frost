CPMAddPackage(
    NAME replxx
    GITHUB_REPOSITORY AmokHuginnsson/replxx
    GIT_TAG release-0.0.4
)

# replxx defines its own `char8_t` typedef for pre-C++20 compatibility, which
# trips GCC's -Wc++20-compat. We compile replxx as-is (its public headers are
# fine), so just silence the warning on the replxx target itself.
if(TARGET replxx)
    target_compile_options(replxx PRIVATE -Wno-c++20-compat)
endif()
