# 使用说明

## 目录结构

```
project/
├── config/
│   └── config.toml  # API 相关配置文件，包含多个 API Key 和其权重信息
├── data/
│   ├── assistant/  # 术语表，存放与术语相关的文件
│   ├── ebook/      # 译文，存放翻译生成的文本文件
│   ├── history/    # 历史记录，存放所有的操作历史记录
│   └── chain/      # 工作流文件夹，存放定义的工作流，文件夹名即为工作流名
└── model_list/      # 存放支持的供应商及其模型的文件夹
```

------

## config文件夹

### 1. config/config.toml

该文件用于配置 API 的密钥信息及其权重。支持为同一供应商配置多个 `api_key`，并为每个 `api_key` 设置一个 `weight`，权重可以是任意整数，`0` 或负数的权重表示不会被使用，权重越大，该 `api_key` 被调用的频率越高。

**示例配置：**

```toml
[OpenAI]
api_keys = [
    {key = "your-openai-api-key-1", weight = 0.7},
    {key = "your-openai-api-key-2", weight = 0.3}
]
base_url = "https://api.openai.com/v1"

[DeepSeek]
api_keys = [
    {key = "api-key1", weight = 1.0},
    {key = "api-key2", weight = 1.0},
    {key = "api-key3", weight = 0},	#不会被使用
    {key = "api-key3", weight = -1} #不会被使用
]
base_url = "https://api.deepseek.com/v1"

[MoonShot]
api_keys = [
    {key = "api-key", weight = 1.0}
]
base_url = "https://api.moonshot.cn/v1"

[SiliconFlow]
api_keys = [
    {key = "api-key", weight = 1.0}
]
base_url = "https://api.siliconflow.cn/v1/chat/completions"

```

------

## data文件夹说明

### 1. assistant 文件夹

存放术语表，术语表用于在翻译过程中提供更准确的术语匹配。

### 2. ebook 文件夹

存放翻译生成的文本。

### 3. history 文件夹

存放翻译的历史记录。

### 4. chain 文件夹

存放工作流，每个工作流为一个文件夹，文件夹名称即为工作流名称， 每个工作流中包含多个 YAML 文件，这些 YAML 文件定义了每个节点的参数和执行顺序。文件命名格式为：

```
{num}_{name}.yaml
```

- **num**：节点的顺序编号，从0开始
- **name**：节点的名称

**示例文件结构：**

```
chain/
└── default/
    ├── 0_TRANSLATION.yaml
    ├── 1_EXPERT_SUGGESTIONS.yaml
    ├── 2_IMPORVE_TRANSLATE.yaml
    ├──......
    └──input.toml
```

**节点参数含义：**

- **stop**：停止生成的序列列表，最多 4 个字符串元素，返回的文本不会包含这些停止序列。
- **n**：模型在一次请求中生成的独立回复数量，注意生成多个回复会消耗更多的 token。
- **frequency_penalty**：用于惩罚 token 的频率的浮点数。较大的值会减少重复的 token 出现，较小的值会增加重复的 token 出现。
- **top_k**：用于限制生成时的候选 token 数量，默认为 50。较小的值减少候选项数量，可能加快生成速度，但降低多样性。
- **top_p**：用于控制采样的方式，取值范围 0 到 1。较大值增加随机性和多样性，较小值增加确定性。
- **stream**：是否启用流式 API，启用时，API 返回一个流式响应。
- **temperature**：控制生成文本的随机性，值越大生成的文本越随机。
- **max_tokens**：最大生成的 token 数量，必须在 2 到 4095 之间。

#### input.toml

用于存放 prompt、源语言、目标语言和待翻译的源文本信息。

**示例配置：**

```toml
[TRANSLATION] #节点名称
system_input = """
你是一位语言专家，专门从事从{source_lang}到{target_lang}的翻译工作。
"""
user_input = """
这是一个从{source_lang}到{target_lang}的翻译，请提供这段文字的{target_lang}翻译，译文中不得出现{source_lang}。
         不要提供任何解释或除翻译外的其他文本。
         {source_text}
"""

[EXPERT_SUGGESTIONS] #节点名称
system_input = """
你的任务是仔细阅读从{source_lang}到{target_lang}的原文和译文，并对译文提出建设性批评和有益的改进建议。
               原文和初始译文由XML标签<SOURCE_TEXT>和<TRANSLATION>分隔，如下所示： <SOURCE_TEXT> {source_text} </SOURCE_TEXT> <TRANSLATION> {TRANSLATION} </TRANSLATION>
               在提出建议时，请注意以下几点，以改进译文的质量：
               (i) 准确性（通过纠正增译、误译、漏译或未翻译的内容），译文中不能含有{source_lang}；
               (ii) 流畅性（遵守{target_lang}的语法、拼写和标点符号规则，并确保没有不必要的重复）；
               (iii) 风格（确保译文反映原文的风格，并考虑文化背景因素）；
               (iv) 术语一致性（确保术语的使用一致，并与原文领域的术语保持一致；如果有习语，则确保使用{target_lang}中的等效表达方式）。
               (vi)注意人称的正确使用，根据原文和上下文确保译文中使用了正确的人称
               请列出具体、有帮助且具有建设性的改进建议，以改进译文。
               每条建议应针对译文中的一个特定部分。
               仅输出建议，其他内容一律不显示。
"""
user_input = """
"""
```

其中：

- **source_lang**：用户输入的原语言
- **target_lang**：用户输入的目标语言
- **source_text**：需要翻译的文本
- **term**：术语表
- **abstract**: 摘要
- **TRANSLATION** ：节点TRANSLATION输出的内容(注意：节点只能使用在它之前的节点输出的内容)

------

## 模型列表

**model_list** 文件夹用于存放支持的供应商及其模型的文件，每个文件包含支持的模型列表。

------

注：由于三个写入操作(assistant、history、ebook)没有原子性，在最后一个节点运行完毕到第一个节点运行之前 关闭程序有概率导致写入的内容出现缺失或者乱序的现象，尽量避免在该时间段关闭程序
