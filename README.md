# MAROL 
**MAROL** is a toolkit designed for generating compilers for different quantum architectures. **MAROL** supplies a language for specifying quantum architectures based on Rust. 

# Dependencies
MAROL requires cargo 1.90.0 and the bash shell. 


# Installation
Clone the repo with 
```
git clone https://github.com/qqq-wisc/qmr-compiler-generator
```

# Usage 
To 

# Notes

Depending on what version of python how and how it is installed, you may need to change the following line in `qmrl`
```
generated-solvers/${base} $3 $4 $5 | python -m json.tool 
```
to 
```
generated-solvers/${base} $3 $4 $5 | python3 -m json.tool 
```
or you can add an alias to your bash config file, which you can find with `echo $SHELL` 
