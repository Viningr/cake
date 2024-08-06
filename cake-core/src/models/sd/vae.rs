use std::fmt::{Debug, Display, Formatter};
use async_trait::async_trait;
use candle_core::{Device, DType, Tensor};
use candle_transformers::models::stable_diffusion::StableDiffusionConfig;
use candle_transformers::models::stable_diffusion::vae::{AutoEncoderKL};
use crate::cake::{Context, Forwarder};
use crate::models::llama3::{Cache};
use crate::models::sd::ModelFile;
use crate::models::sd::util::{get_device, get_sd_config, pack_tensors, unpack_tensors};
use crate::StableDiffusionVersion;

pub struct VAE {
    vae_model: AutoEncoderKL
}

impl Debug for VAE {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl Display for VAE {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

#[async_trait]
impl Forwarder for VAE {
    fn load(_name: String, ctx: &Context) -> anyhow::Result<Box<Self>>
    where
        Self: Sized
    {
        let dtype = if ctx.args.sd_args.use_f16 { DType::F16 } else { DType::F32 };
        let device = get_device(ctx.args.cpu)?;

        let sd_config = get_sd_config(ctx)?;

        Self::load_model(
            ctx.args.sd_args.vae.clone(),
            ctx.args.sd_args.sd_version,
            ctx.args.sd_args.use_f16,
            &device,
            dtype,
            &sd_config,
        )
    }

    async fn forward(&self, x: &Tensor, index_pos: usize, block_idx: usize, cache: &mut Cache) -> anyhow::Result<Tensor> {
        let unpacked_tensors = unpack_tensors(x)?;

        let direction:i64 = unpacked_tensors.get(0).unwrap().to_scalar()?;
        let input = unpacked_tensors.get(1).unwrap();

        if direction == 1 {
            let dist = self.vae_model.encode(input)?;
            Ok(dist.sample()?)
        } else {
            Ok(self.vae_model.decode(input)?)
        }
    }

    async fn forward_mut(&mut self, x: &Tensor, index_pos: usize, block_idx: usize, cache: &mut Cache) -> anyhow::Result<Tensor> {
        self.forward(x, index_pos, block_idx, cache).await
    }

    fn layer_name(&self) -> &str {
        "vae"
    }
}

impl VAE {
    pub fn load_model(name: Option<String>, version: StableDiffusionVersion, use_f16: bool, device: &Device, dtype: DType, config: &StableDiffusionConfig) -> anyhow::Result<Box<Self>>
    where
        Self: Sized {
        let vae_weights = ModelFile::Vae.get(name, version, use_f16)?;
        let vae_model = config.build_vae(vae_weights, device, dtype)?;

        Ok(Box::new(Self{
            vae_model,
        }))
    }

    pub async fn encode(forwarder: &Box<dyn Forwarder>, image: Tensor, device: &Device, cache: &mut Cache) -> anyhow::Result<Tensor> {
        let tensors = Vec::from([
            Tensor::from_slice(&[1i64], 1, device)?,
            image
        ]);

        let combined_tensor = pack_tensors(tensors, device)?;

        Ok(forwarder.forward(&combined_tensor, 0, 0, cache).await?)
    }

    pub async fn decode(forwarder: &Box<dyn Forwarder>, latents: Tensor, device: &Device, cache: &mut Cache) -> anyhow::Result<Tensor> {
        let tensors = Vec::from([
            Tensor::from_slice(&[0i64], 1, device)?,
            latents,
        ]);

        let combined_tensor = pack_tensors(tensors, device)?;

        let result = forwarder.forward(&combined_tensor, 0, 0, cache).await?;
        Ok(result)
    }
}