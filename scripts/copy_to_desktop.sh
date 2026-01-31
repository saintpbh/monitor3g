#!/bin/bash
# Check if build/bin/Monitor3G.app exists
if [ -d "build/bin/Monitor3G.app" ]; then
    cp -r build/bin/Monitor3G.app ~/Desktop/
    echo "Copied Monitor3G.app to Desktop"
elif [ -d "build/Monitor3G.app" ]; then
    cp -r build/Monitor3G.app ~/Desktop/
    echo "Copied Monitor3G.app to Desktop"
else
    echo "Error: Monitor3G.app not found in build/bin or build/"
fi

