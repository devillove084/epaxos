# EPaxos Rebuild

## 代码结构

> ├── Cargo.toml 
> ├── **client** 
> │  ├── Cargo.toml 
> │  └── **src** 
> ├── README.md 
> ├── **server** 
> │  ├── build.rs 
> │  ├── Cargo.toml 
> │  ├── epaxos.proto 
> │  └── **src** 
> └── **tests**

* client是对EPaxos集群的使用，展示了如何使用异步的方式来构建Read/Write;
* server是整个集群的一个Replica节点，构建集群。



## 如何使用

1. Clone到本地并Cargo build;

2. 在target/debug下，分别执行(可以开三个终端)

   > * ./server 127.0.0.1 10000 0 1 2
   > * ./server 127.0.0.1 10001 1 0 2
   > * ./server 127.0.0.1 10002 2 1 0

3. 然后执行client

   > * ./client 127.0.0.1 10000

这里稍微解释一下参数，server的前面两个参数为replica绑定的ip&&port，后面的三个参数第一个参数为自己的id，后面两个为要去链接的replicaID。

client的参数为要链接的目标，这里填三个中的任何一个即可。