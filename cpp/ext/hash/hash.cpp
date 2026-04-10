#include <frost/extensions-common.hpp>

#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <openssl/evp.h>

#include <zlib.h>

#include <boost/algorithm/string/case_conv.hpp>
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

Value_Ptr hmac_string(const std::string& algorithm, const String& key,
                      const String& data)
{
    EVP_MAC* mac = EVP_MAC_fetch(nullptr, "HMAC", nullptr);
    BOOST_SCOPE_EXIT_ALL(&)
    {
        EVP_MAC_free(mac);
    };
    EVP_MAC_CTX* ctx = EVP_MAC_CTX_new(mac);
    BOOST_SCOPE_EXIT_ALL(&)
    {
        EVP_MAC_CTX_free(ctx);
    };

    OSSL_PARAM params[] = {
        // This const_cast is fine, I promise
        // OpenSSL just has the wrong signature here
        OSSL_PARAM_construct_utf8_string(
            "digest", const_cast<char*>(algorithm.c_str()), 0),
        OSSL_PARAM_END,
    };

    EVP_MAC_init(ctx, reinterpret_cast<const unsigned char*>(key.data()),
                 key.size(), params);
    EVP_MAC_update(ctx, reinterpret_cast<const unsigned char*>(data.data()),
                   data.size());

    unsigned char buf[EVP_MAX_MD_SIZE];
    std::size_t len;

    EVP_MAC_final(ctx, buf, &len, sizeof(buf));

    return Value::create(
        fmt::format("{:02x}", fmt::join(std::span{buf, len}, "")));
}

} // namespace

BUILTIN(crc32)
{
    REQUIRE_ARGS("hash.crc32", PARAM("input", TYPES(String)));

    const auto& input = GET(0, String);
    auto checksum =
        ::crc32(::crc32(0L, Z_NULL, 0),
                reinterpret_cast<const Bytef*>(input.data()), input.size());
    return Value::create(fmt::format("{:08x}", checksum));
}

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

namespace hmac
{

#define X(ALG)                                                                 \
    BUILTIN(ALG)                                                               \
    {                                                                          \
        REQUIRE_ARGS("hash.hmac." #ALG, PARAM("key", TYPES(String)),           \
                     PARAM("input", TYPES(String)));                           \
        return hmac_string(boost::algorithm::to_upper_copy(std::string{#ALG}), \
                           GET(0, String), GET(1, String));                    \
    }

X_HASH_ALGS

#undef X
} // namespace hmac

#define X(ALG) NS_ENTRY(hmac, ALG),
static const auto hmac_map = Value::create(Value::trusted, Map{X_HASH_ALGS});
#undef X

} // namespace hash

#define X(ALG) ENTRY(ALG),

REGISTER_EXTENSION(hash, ENTRY(crc32), {"hmac"_s, hmac_map}, X_HASH_ALGS)
#undef X

} // namespace frst
