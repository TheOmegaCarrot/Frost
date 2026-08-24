#ifndef FROST_TEST_HELPERS_DUMMY_CALLABLE_HPP
#define FROST_TEST_HELPERS_DUMMY_CALLABLE_HPP
// AI-generated test helper by Codex (GPT-5).
// Signed: Codex (GPT-5).

#include <frost/value.hpp>

namespace frst::testing
{
struct Dummy_Callable final : frst::Callable
{
    frst::Value_Ptr call(std::span<const frst::Value_Ptr>) const override
    {
        return frst::Value::null();
    }

    std::string debug_dump() const override
    {
        return "<dummy>";
    }

    std::string name() const override
    {
        return "<dummy>";
    }
};
} // namespace frst::testing

#endif
