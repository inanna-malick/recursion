set -e
cd ./recursion
cargo doc --no-deps
cd ..
rm -rf ./docs
echo "<meta http-equiv=\"refresh\" content=\"0; url=recursion\">" > target/doc/index.html
cp -r target/doc ./docs
