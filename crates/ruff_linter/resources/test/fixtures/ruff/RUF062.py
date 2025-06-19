from transformers.models.glm.configuration_glm import GlmConfig
from transforemrs.models.llama.modeling_llama import LlamaAttention


class GlmAttention(LlamaAttention):
    def __init__(self, config: GlmConfig, layer_idx: int | None = None):
        super().__init__(config, layer_idx)
        self.o_proj = None

