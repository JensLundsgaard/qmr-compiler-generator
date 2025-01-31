#!/bin/bash
path=$2
file=${path##*/}  
base=${file%.qmrl}

if [ "$1" == "compile" ]; then
    QMRL_PATH=$path cargo build --release --bin generator
    cp target/release/generator generated-solvers/${base}

elif [ "$1" == 'debug' ]; then
    QMRL_PATH=$path cargo build --bin generator

elif [ "$1" == "run" ];
     then

        # run the program
        generated-solvers/${base} $3 $4 $5 | python -m json.tool 
    else
        echo "Invalid argument. Please use 'debug', 'run' or 'compile'"
fi