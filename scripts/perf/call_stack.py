import json
import copy
import sys

def name_of_function(itable:dict, func_index):
    return itable[func_index][0]["function_name"]

def name_of_opcode(opcode):
    if not type(opcode) == str:
        if len(opcode.items()) != 1:
            print("Panic")
            exit(1)

        for key, value in opcode.items():
            opcode = key
            break

    return opcode

def analyze_etable(itable:dict, etable):
    dict = {}
    frame = []

    for entry in etable:
        inst = itable[entry["fid"]][entry["iid"]]
        opcode = name_of_opcode(inst["opcode"])

        if len(frame) == 0:
            frame.append(inst["function_name"])

        if tuple(frame) not in dict:
                dict[tuple(frame)] = 1
        else:
                dict[tuple(frame)] = dict[tuple(frame)] + 1

        if opcode == "Call":
            frame.append(name_of_function(itable, entry["step_info"]["Call"]["index"]))
        elif opcode == "CallIndirect":
            frame.append(name_of_function(itable, entry["step_info"]["CallIndirect"]))
        elif opcode == "Return":
            frame.pop()
    
    return dict

def generate_log(dict):
    for key, value in dict.items():
        print(*key, sep=";", end="")
        print(" ", end="")
        print(value)

def main():
    if len(sys.argv) != 3:
        print("Usage: python call_stack.py <Path of itable.json> <Path of etable.json>")
        exit(1)

    itable_path = sys.argv[1]
    etable_path = sys.argv[2]

    itable = open(itable_path)
    itable = json.load(itable)
    etable = open(etable_path)
    etable = json.load(etable)

    dict = analyze_etable(itable, etable)
    generate_log(dict)

main()
