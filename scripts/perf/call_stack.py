import json
import copy
import sys

def analyze_itable(itable):
    dict = {}

    for entry in itable:
        dict[(entry["fid"], entry["iid"])] = entry

    return dict

def name_of_function(itable:dict, func_index):
    return itable[func_index, 0]["function_name"]

def analyze_etable(itable:dict, etable):
    dict = {}
    frame = []

    for entry in etable:
        inst = itable[(entry["fid"], entry["iid"])]

        if len(frame) == 0:
            frame.append(inst["function_name"])

        if not type(inst["opcode"]) == str:
            if len(inst["opcode"].items()) != 1:
                print("Panic")
                exit(1)

            for key, value in inst["opcode"].items():
                opcode = key
                break

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

    itable = analyze_itable(itable)

    dict = analyze_etable(itable, etable)
    generate_log(dict)

main()
