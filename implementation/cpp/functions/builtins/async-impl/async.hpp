#ifndef FROST_BUILTINS_ASYNC_HPP
#define FROST_BUILTINS_ASYNC_HPP

#include <frost/value.hpp>

#include <boost/asio/io_context.hpp>

#include <future>
#include <thread>

namespace frst::async
{

template <std::movable Result, std::invocable<Result&&> auto Translate>
    requires std::is_same_v<Value_Ptr,
                            std::invoke_result_t<decltype(Translate), Result&&>>
struct Task
{
    boost::asio::io_context ioc;
    std::future<Result> future;
    std::jthread worker;

    std::atomic<bool> complete = false;
    std::once_flag cache_once;
    Value_Ptr cache;

    Value_Ptr get()
    {
        std::call_once(cache_once, [&] {
            Result result = future.get();
            cache = Translate(std::move(result));
            complete = true;
        });
        return cache;
    }
    Value_Ptr is_ready()
    {
        return Value::create(complete.load());
    }
};

} // namespace frst::async

#endif
