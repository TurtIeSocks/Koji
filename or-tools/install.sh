#!/bin/bash
# add output as the first arg
os=$OSTYPE
cpu=$(uname -p)
minor_version="9.10"
patch_version="4067"
full_version="${minor_version}.${patch_version}"

echo Installing or-tools v${full_version}

if [[ $PWD != *"or-tools"* ]]; then
        cd ./or-tools
fi

echo Found $os $cpu

if [ ! -d "${full_version}" ]; then
        url=""
        if [[ "$os" == "linux-gnu"* ]]; then
                distro=$(cat /etc/os-release | grep "^ID=" | cut -d '=' -f 2)
                version=$(cat /etc/os-release | grep "^VERSION_ID=" | cut -d '=' -f 2)
                distro=${distro//\"/}
                version=${version//\"/}
                echo Found distro $distro $version
                url="https://github.com/google/or-tools/releases/download/v${minor_version}/or-tools_amd64_${distro}-${version}_cpp_v${full_version}.tar.gz"
        elif [[ "$os" == "darwin"* ]]; then
                if [[ "$cpu" == "arm" ]]; then
                        cpu="arm64"
                fi
                url="https://github.com/google/or-tools/releases/download/v${minor_version}/or-tools_${cpu}_macOS-14.4.1_cpp_v${full_version}.tar.gz"
        else
                echo "Unsupported OS, contact the developer"
                exit 1
        fi
        echo Downloading $url
        curl -L $url -o ortools.tar.gz
        tar -xzf ortools.tar.gz
        mv ./or-tools_* ./${full_version}
        rm ortools.tar.gz
fi

# check if folder exists and remove if so
if [ -d "./${full_version}/examples/koji_tsp" ]; then
        rm -r ./${full_version}/examples/koji_tsp
fi
cp -r ./src/tsp ./${full_version}/examples/koji_tsp
cd $full_version
make build SOURCE=examples/koji_tsp

if [ $# -eq 0 ]; then
        echo hi
        # mv ./examples/koji_tsp/build/bin/koji_tsp ../../server/algorithms/src/routing/plugins/tsp
        # else
        # mv ./${full_version}/examples/koji_tsp/build/bin/koji_tsp /algorithms/src/routing/plugins/tsp
fi
