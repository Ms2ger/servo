set -e
cd build; ../configure
make tidy
make -j2
