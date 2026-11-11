#!/bin/bash

# A program which generates input data for the elevator simulation
# Usage: ./generateInputData.sh
#
# This script generates input data for the elevator simulation and saves it to a csv file.
#
# Example:
# ./generateInputData.sh > input.csv
# 
# Output file row format:
# time: Integer time step when request is made
# id: unique passenger identifier
# source: origin floor
# dest: destination floor
#
# For example:
# time,id,source,dest
# 0,passenger1,1,51
# 0,passenger2,1,37
# 10,passenger3,20,1

