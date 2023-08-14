from z3 import *
from utils import *
from functools import reduce

s = init_z3_solver()

# define spec
lhs, rhs, res = Ints('lhs rhs res')
s.add(is_i64(lhs))
s.add(is_i64(rhs))

wasm_mul_i64 = Function('WasmMulI64', IntSort(), IntSort(), IntSort())
s.add(ForAll([lhs, rhs], wasm_mul_i64(lhs, rhs) == (lhs * rhs) % I64_MODULUS))

# define var
aux = Int('aux')
intermediate1, intermediate2 = Ints('intermediate1 intermediate2')

constrain = Function('Constrain', IntSort(), IntSort(), IntSort(), BoolSort())
constraints = [
    is_i64(aux),
    is_i64(res),
    # c.bin.mul
    intermediate1 == fr_mul(rhs, lhs),
    intermediate2 == fr_mul(aux, I64_MODULUS),
    fr_sub(fr_sub(intermediate1, intermediate2), res) == 0
]

s.add(ForAll([aux, intermediate1, intermediate2], And(constrain(aux, intermediate1, intermediate2) == reduce(
    lambda x, y: And(x, y), constraints))))

s.push()
# Soundness
s.add(And(constrain(aux, intermediate1,
      intermediate2), wasm_mul_i64(lhs, rhs) != res))

check_res = s.check()
print('--------------Soundness---------------')
if check_res.r == Z3_L_TRUE:
    print('Verify: Fail')
    print(s.model())
elif check_res.r == Z3_L_FALSE:
    print('Verify: Pass')
else:
    print('Verify: Fail')
s.pop()


s.push()
# Completeness
s.add(And(wasm_mul_i64(lhs, rhs) == res, ForAll(
    [aux, intermediate1, intermediate2], Not(constrain(aux, intermediate1, intermediate2)))))

check_res = s.check()
print('-------------Completeness----------------')
if check_res.r == Z3_L_TRUE:
    print('Verify: Fail')
    print(s.model())
elif check_res.r == Z3_L_FALSE:
    print('Verify: Pass')
else:
    print('Verify: Fail')
s.pop()
