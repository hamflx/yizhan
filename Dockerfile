FROM rust as build

WORKDIR /usr/src/yizhan-node

COPY rust-toolchain .
COPY ./.cargo ./.cargo
COPY Cargo.lock /usr/src/yizhan-node/Cargo.lock
COPY Cargo.toml /usr/src/yizhan-node/Cargo.toml
COPY packages/yizhan-bootstrap/Cargo.toml /usr/src/yizhan-node/packages/yizhan-bootstrap/Cargo.toml
COPY packages/yizhan-node/Cargo.toml /usr/src/yizhan-node/packages/yizhan-node/Cargo.toml
COPY packages/yizhan-plugin/Cargo.toml /usr/src/yizhan-node/packages/yizhan-plugin/Cargo.toml
COPY packages/yizhan-plugin-wechat/Cargo.toml /usr/src/yizhan-node/packages/yizhan-plugin-wechat/Cargo.toml
COPY packages/yizhan-protocol/Cargo.toml /usr/src/yizhan-node/packages/yizhan-protocol/Cargo.toml

RUN bash -c 'printf "\n[source.crates-io]\nreplace-with = \"ustc\"\n[source.ustc]\nregistry = \"git://mirrors.ustc.edu.cn/crates.io-index\"\n" >>/usr/src/yizhan-node/.cargo/config.toml'
RUN bash -c 'mkdir -p /usr/src/yizhan-node/packages/{yizhan-bootstrap,yizhan-node,yizhan-plugin,yizhan-plugin-wechat,yizhan-protocol}/src'
RUN bash -c 'echo "fn main() {}" >/usr/src/yizhan-node/packages/yizhan-node/src/main.rs'
RUN bash -c 'echo "#[no_mangle] pub fn test() {}" | tee /usr/src/yizhan-node/packages/{yizhan-bootstrap,yizhan-plugin,yizhan-plugin-wechat,yizhan-protocol}/src/lib.rs'
RUN cargo build --release

COPY . .
RUN bash -c 'printf "\n[source.crates-io]\nreplace-with = \"ustc\"\n[source.ustc]\nregistry = \"git://mirrors.ustc.edu.cn/crates.io-index\"\n" >>/usr/src/yizhan-node/.cargo/config.toml'
RUN cargo clean && cargo build --release

FROM rust
COPY --from=build /usr/src/yizhan-node/target/release/yizhan-node /bin/yizhan-node
ENTRYPOINT ["/bin/yizhan-node"]
