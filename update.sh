git submodule update --remote --merge finam-trade-api
git add finam-trade-api
git submodule update --remote --merge googleapis
git add googleapis

cd generator

cargo run
