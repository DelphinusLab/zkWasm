from z3 import *

Fr = 16798108731015832284940804142231733909759579603404752749028378864165570215949
I64_MODULUS = 1 << 64
I32_MODULUS = 1 << 32
MAX_COMMON_RANGE = 1 << 27

fr_add = Function('FieldAdd', IntSort(), IntSort(), IntSort())
fr_sub = Function('FieldSub', IntSort(), IntSort(), IntSort())
fr_mul = Function('FieldMul', IntSort(), IntSort(), IntSort())

is_bit = Function('IsBit', IntSort(), BoolSort())
is_i32 = Function('IsI32', IntSort(), BoolSort())
is_i64 = Function('IsI64', IntSort(), BoolSort())
is_field = Function('IsField', IntSort(), BoolSort())
is_common_range = Function('IsCommonRange', IntSort(), BoolSort())

def init_z3_solver():
    s = Solver()

    lhs, rhs = Ints('_lhs _rsh')
    s.add(ForAll([lhs, rhs], fr_sub(lhs, rhs) == (lhs - rhs) % Fr))
    s.add(ForAll([lhs, rhs], fr_mul(lhs, rhs) == (lhs * rhs) % Fr))
    s.add(ForAll([lhs, rhs], fr_add(lhs, rhs) == (lhs + rhs) % Fr))
    s.add(ForAll([lhs], is_bit(lhs) == And(lhs >= 0, lhs <= 1)))
    s.add(ForAll([lhs], is_i32(lhs) == And(lhs >= 0, lhs < I32_MODULUS)))
    s.add(ForAll([lhs], is_i64(lhs) == And(lhs >= 0, lhs < I64_MODULUS)))
    s.add(ForAll([lhs], is_field(lhs) == And(lhs >= 0, lhs < Fr)))
    s.add(ForAll([lhs], is_common_range(lhs) == And(lhs >= 0, lhs <= MAX_COMMON_RANGE)))

    return s