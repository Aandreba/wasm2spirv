cd /usr/bin
URL=$(curl -s https://ziglang.org/download/index.json | jq -r '.master."x86_64-linux".tarball')
wget -c $URL | tar -xj
