# eRPC for Rust

eRPC for Rust.

It's built on top of
[eRPC in C++](https://github.com/erpc-io/eRPC) via [autocxx](https://github.com/google/autocxx).

This lib is still in the stage of prove-of-concept and under heavy development.

## Prepare
   * [rdma-core](https://github.com/linux-rdma/rdma-core/tree/stable-v40) must be installed from source. 
     We recommend the tag `stable-v40`. First, install its dependencies listed in rdma-core's README.
      Then, in the `rdma-core` directory:
       * `cmake .`
       * `sudo make install`
   * Install upstream pre-requisite libraries and modules:
       * `sudo apt install make cmake g++ gcc libnuma-dev libgflags-dev numactl libbsd-dev meson libjansson-dev ninja-build`
       * `sudo modprobe ib_uverbs`
       * `sudo modprobe mlx4_ib`

   *  Create hugepages:
```bash
sudo bash -c "echo 2048 > /sys/devices/system/node/node0/hugepages/hugepages-2048kB/nr_hugepages"
sudo mkdir /mnt/huge
sudo mount -t hugetlbfs nodev /mnt/huge
```
## Build throughput benchmark tool
```bash
cd erpc-rs/apps/large_rpc_tput && cargo build --release # see eRPC's scripts/do.sh to learn how to run
```

## Run examples
```bash
cd erpc-rs/examples/hello_world && cargo build --release
sudo ./target/release/hello_server # or hello_server_async or hello_server_async2
sudo ./hello_client # on the other machine
```

