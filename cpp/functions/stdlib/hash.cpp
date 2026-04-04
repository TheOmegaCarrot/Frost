#include <frost/builtins-common.hpp>

#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <openssl/evp.h>

#include <boost/scope_exit.hpp>

#include <fmt/ranges.h>

namespace frst
{

namespace hash
{

namespace
{
Value_Ptr hash_string(const EVP_MD* md, const String& input)
{
    auto* ctx = EVP_MD_CTX_new();
    BOOST_SCOPE_EXIT_ALL(&)
    {
        EVP_MD_CTX_free(ctx);
    };
    EVP_DigestInit_ex(ctx, md, nullptr);
    EVP_DigestUpdate(ctx, input.data(), input.size());
    unsigned char buf[EVP_MAX_MD_SIZE];
    unsigned int len;
    EVP_DigestFinal_ex(ctx, buf, &len);
    return Value::create(
        fmt::format("{:02x}", fmt::join(std::span{buf, len}, "")));
}
} // namespace

#define X_HASH_ALGS                                                            \
    X(md5)                                                                     \
    X(sha1)                                                                    \
    X(sha224)                                                                  \
    X(sha256)                                                                  \
    X(sha384)                                                                  \
    X(sha512)                                                                  \
    X(sha3_256)                                                                \
    X(sha3_512)                                                                \
    X(blake2s256)                                                              \
    X(blake2b512)                                                              \
    X(ripemd160)                                                               \
    X(sha3_224)                                                                \
    X(sha3_384)                                                                \
    X(sha512_224)                                                              \
    X(sha512_256)                                                              \
    X(sm3)

#define X(ALG)                                                                 \
    BUILTIN(ALG)                                                               \
    {                                                                          \
        REQUIRE_ARGS("hash." #ALG, PARAM("input", TYPES(String)));             \
        return hash_string(EVP_##ALG(), GET(0, String));                       \
    }

X_HASH_ALGS

#undef X

} // namespace hash

#define X(ALG) ENTRY(ALG),

STDLIB_MODULE(hash, X_HASH_ALGS)
#undef X

} // namespace frst
