# 基于 Dify 的 API 接口翻译工具

基于 Dify 提供的 API 接口，一个简单的翻译工具。

## 部署 Dify

确保您已安装了 Docker 和 Docker Compose。参考 [Dify 官方文档](https://docs.dify.ai/zh-hans/getting-started/install-self-hosted/docker-compose) 进行部署。

###  接入模型供应商

完成 Docker 部署后，您需要在 Dify 中接入模型供应商。参考 Dify 文档中的配置说明，根据需要选择合适的模型供应商。

### 创建工作流应用

进入 Dify 工作室，创建或导入工作流应用。

- 登录到 Dify 工作室。
- 创建一个新的工作流应用，或导入一个已有的应用。
- 配置工作流，并点击发布。

### 获取 API 密钥

在工作流应用发布之后，您需要生成一个 API 密钥。

- 进入应用的监测页面。
- 创建并获取 API 密钥。

### one-hub

想使用one-hub作为Dify的模型供应商，需要将Dify的docker-compose.yaml替换为上面的docker-compose.yaml，在Dify的模型供应商中找到OpenAI兼容，base_url为http://one-hub:3000/v1。

