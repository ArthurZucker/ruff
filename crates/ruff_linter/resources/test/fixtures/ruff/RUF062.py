from .configuration_glm import GlmConfig


class LlamaAttention:
    def __init__(self, config: GlmConfig, layer_idx: int | None = None):
        self.o_proj = config.hidden_size

    def forward(self, x):
        return x


class GlmAttention(LlamaAttention):
    def __init__(self, config: GlmConfig, layer_idx: int | None = None):
        super().__init__(config, layer_idx)
        self.o_proj = None

