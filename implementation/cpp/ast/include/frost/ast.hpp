#ifndef FROST_AST_HPP
#define FROST_AST_HPP

// Amalgam header including all AST components

#include <frost/ast/array-constructor.hpp>
#include <frost/ast/array-destructure.hpp>
#include <frost/ast/binop.hpp>
#include <frost/ast/define.hpp>
#include <frost/ast/expression.hpp>
#include <frost/ast/filter.hpp>
#include <frost/ast/foreach.hpp>
#include <frost/ast/format-string.hpp>
#include <frost/ast/function_call.hpp>
#include <frost/ast/if.hpp>
#include <frost/ast/index.hpp>
#include <frost/ast/literal.hpp>
#include <frost/ast/map-constructor.hpp>
#include <frost/ast/map-destructure.hpp>
#include <frost/ast/map.hpp>
#include <frost/ast/name_lookup.hpp>
#include <frost/ast/reduce.hpp>
#include <frost/ast/statement.hpp>
#include <frost/ast/unop.hpp>

#if __has_include(<frost/ast/lambda.hpp>)
#include <frost/ast/lambda.hpp>
#endif

#endif
