#!/bin/bash

os=$OSTYPE
cpu=$(uname -p)

if [[ $PWD != *"or-tools"* ]]; then
        cd ./or-tools
fi

echo Found $os $cpu

if [ ! -d "source" ]; then
        url=""
        if [[ "$os" == "linux-gnu"* ]]; then
                distro=$(cat /etc/os-release | grep "^ID=" | cut -d '=' -f 2)
                version=$(cat /etc/os-release | grep "^VERSION_ID=" | cut -d '=' -f 2)
                distro=${distro//\"/}
                version=${version//\"/}
                echo Found distro $distro $version
                url="https://github.com/google/or-tools/releases/download/v9.5/or-tools_amd64_${distro}-${version}_cpp_v9.5.2237.tar.gz"
        elif [[ "$os" == "darwin"* ]]; then
                if [[ "$cpu" == "arm" ]]; then
                        cpu="arm64"
                fi
                url="https://github.com/google/or-tools/releases/download/v9.5/or-tools_${cpu}_macOS-13.0.1_cpp_v9.5.2237.tar.gz"
        else
                echo "Unsupported OS, contact the developer"
                exit 1
        fi
        echo Downloading $url
        curl -L $url -o ortools.tar.gz
        tar -xzf ortools.tar.gz
        mv ./or-tools_* ./source
        rm ortools.tar.gz
fi
cd source
mkdir examples/koji
cp ../tsp/tsp.cc ./examples/koji/koji.cc
cp ../tsp/CMakeLists.txt ./examples/koji/CMakeLists.txt
make build SOURCE=examples/koji/koji.cc
mv ./examples/koji/build/bin/koji ../../server/algorithms/src/routing/plugins/tsp
