# EPaxos Rebuild

## 代码结构

* client是对EPaxos集群的使用，展示了如何使用异步的方式来构建Read/Write;
* server是整个集群的一个Replica节点，构建集群。



## Features

1. 使用grpcio(PingCAP)构建，异步支持明确;
2. 使用smol构建内部异步函数，异步覆盖完善;
3. 替原作者补全Commit逻辑；
4. 使用Tarjan算法补全Execute逻辑(Experiment)
5. 构建测试代码



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

## 下一步工作

2. 考虑使用并发数据结构，替代现在的hashmap;
2. Epoch(翻译成版本，或者世代)
3. MemberShip Change；
4. Failover Revovery；
5. Log存储重新设计，内存数据结构，已经Flush Executed log on disk;
6. 将WriteRequest重新设计为trait，可以暴露为接口自定义请求；
7. …….