#!/bin/bash
echo "Starting debug script at $(date)" > debug_output.txt
echo "User: $(whoami)" >> debug_output.txt
echo "PWD: $(pwd)" >> debug_output.txt
echo "Docker Path: $(which docker)" >> debug_output.txt
docker ps >> debug_output.txt 2>&1
echo "Docker exit code: $?" >> debug_output.txt
echo "Done" >> debug_output.txt
