#ifndef FROST_AST_HPP
#define FROST_AST_HPP

// Amalgam header including all AST components

#include <frost/ast/array-constructor.hpp>
#include <frost/ast/binop.hpp>
#include <frost/ast/define.hpp>
#include <frost/ast/destructure-array.hpp>
#include <frost/ast/destructure-binding.hpp>
#include <frost/ast/destructure-map.hpp>
#include <frost/ast/destructure.hpp>
#include <frost/ast/do.hpp>
#include <frost/ast/expression.hpp>
#include <frost/ast/filter.hpp>
#include <frost/ast/foreach.hpp>
#include <frost/ast/format-string.hpp>
#include <frost/ast/function-call.hpp>
#include <frost/ast/if.hpp>
#include <frost/ast/index.hpp>
#include <frost/ast/literal.hpp>
#include <frost/ast/map-constructor.hpp>
#include <frost/ast/map.hpp>
#include <frost/ast/match-alternative.hpp>
#include <frost/ast/match-array.hpp>
#include <frost/ast/match-binding.hpp>
#include <frost/ast/match-map.hpp>
#include <frost/ast/match-pattern.hpp>
#include <frost/ast/match-value.hpp>
#include <frost/ast/name-lookup.hpp>
#include <frost/ast/reduce.hpp>
#include <frost/ast/statement.hpp>
#include <frost/ast/unop.hpp>

#if __has_include(<frost/ast/lambda.hpp>)
#include <frost/ast/lambda.hpp>
#endif

#endif
