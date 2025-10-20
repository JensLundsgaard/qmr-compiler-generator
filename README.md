# MAROL 
**MAROL** is a toolkit designed for generating compilers for different quantum architectures. **MAROL** supplies a language for specifying quantum architectures based on Rust. 

# Dependencies
MAROL requires **Cargo** 1.90.0, **Python** 3.8 and a bash shell. 


# Installation
Clone the repo with 
```
git clone https://github.com/qqq-wisc/qmr-compiler-generator
```
and build with
```
cargo build
```

# Usage 
To compile a `.qmrl` file run
```
./qmrl compile $FILE
```
where $FILE is the relative path to your MAROL file ending in `.qmrl`. The compiled program will be at `generated-solvers/$BASE` where $BASE is the filename without the `.qmrl` ending.

To debug a `.qmrl` file run 
```
./qmrl debug $FILE
```
where $FILE is the relative path to your MAROL file ending in `.qmrl`. The debug output will be at `generator/debug`

To run a `.qmrl` run
```
./qmrl run $FILE <circuit> <graph> --<solve-mode>
```
where $FILE is the relative path to your MAROL file ending in `.qmrl`. 

# Notes
Depending on what version of python how and how it is installed, you may need to add an alias to your bash config file, which should be at either `~/.bashrc`, `~/.bash\_profile` or `~/.zshrc`

# References 
MAROL is based on the following paper:
[1] Abtin Molavi, Amanda Xu, Ethan Cecchetti, Swamit Tannu, Aws Albarghouthi. "[Generating Compilers for Qubit Mapping and Routing](https://arxiv.org/pdf/2508.10781)" 

