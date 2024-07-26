# transfer zksync key to halo2 key

The zkSync keys are divided into two files: one for the monomial key and one for the Lagrange key. 

the halo2 key is a single file that contains both the monomial and Lagrange.

Therefore, a tool is needed to convert the two zkSync key files into a single halo2 key file.
## Features

- Converts zkSync key to halo2 key

## Usage

To run the tool, use the following command:

```sh
cargo run -- ./setup_2^22.key ./setup_2^22_lagrange.key ./K22.params

Three input parameters are required:
1.The path to the monomial key.
2.The path to the lagrange key.
3.The path to the output halo2 key.”