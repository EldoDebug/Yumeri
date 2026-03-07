use std::os::fd::OwnedFd;

use wayland_server::protocol::{wl_shm, wl_shm_pool};
use wayland_server::{Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, New, Resource};

use crate::compositor::{CompositorState, ShmBufferSpec, ShmPoolState};

impl GlobalDispatch<wl_shm::WlShm, ()> for CompositorState {
    fn bind(
        _state: &mut Self,
        _dh: &DisplayHandle,
        _client: &Client,
        resource: New<wl_shm::WlShm>,
        _global_data: &(),
        data_init: &mut DataInit<'_, Self>,
    ) {
        let shm = data_init.init(resource, ());
        shm.format(wl_shm::Format::Argb8888);
        shm.format(wl_shm::Format::Xrgb8888);
    }
}

impl Dispatch<wl_shm::WlShm, ()> for CompositorState {
    fn request(
        state: &mut Self,
        _client: &Client,
        _resource: &wl_shm::WlShm,
        request: wl_shm::Request,
        _data: &(),
        _dh: &DisplayHandle,
        data_init: &mut DataInit<'_, Self>,
    ) {
        if let wl_shm::Request::CreatePool { id, fd, size } = request {
            let pool = data_init.init(id, ());
            let pool_id = pool.id();

            match create_pool_state(fd, size as usize) {
                Ok(pool_state) => {
                    state.shm_pools.insert(pool_id, pool_state);
                }
                Err(e) => {
                    log::error!("Failed to mmap SHM pool: {e}");
                }
            }
        }
    }
}

fn create_pool_state(fd: OwnedFd, size: usize) -> std::io::Result<ShmPoolState> {
    use std::os::fd::AsRawFd;
    let raw_fd = fd.as_raw_fd();
    let mmap = unsafe { memmap2::MmapOptions::new().len(size).map_mut(raw_fd)? };
    Ok(ShmPoolState { mmap, fd, size })
}

impl Dispatch<wl_shm_pool::WlShmPool, ()> for CompositorState {
    fn request(
        state: &mut Self,
        _client: &Client,
        resource: &wl_shm_pool::WlShmPool,
        request: wl_shm_pool::Request,
        _data: &(),
        _dh: &DisplayHandle,
        data_init: &mut DataInit<'_, Self>,
    ) {
        let pool_id = resource.id();
        match request {
            wl_shm_pool::Request::CreateBuffer {
                id,
                offset,
                width,
                height,
                stride,
                format,
            } => {
                let buffer = data_init.init(id, ());
                let format = match format {
                    wayland_server::WEnum::Value(f) => f,
                    _ => wl_shm::Format::Argb8888,
                };
                let spec = ShmBufferSpec {
                    pool_id: pool_id.clone(),
                    offset,
                    width,
                    height,
                    stride,
                    format,
                };
                state.shm_buffers.insert(buffer.id(), spec);
            }
            wl_shm_pool::Request::Resize { size } => {
                if let Some(pool) = state.shm_pools.get_mut(&pool_id) {
                    let new_size = size as usize;
                    if new_size > pool.size {
                        use std::os::fd::AsRawFd;
                        let raw_fd = pool.fd.as_raw_fd();
                        match unsafe { memmap2::MmapOptions::new().len(new_size).map_mut(raw_fd) } {
                            Ok(new_mmap) => {
                                pool.mmap = new_mmap;
                                pool.size = new_size;
                            }
                            Err(e) => {
                                log::error!("Failed to resize SHM pool: {e}");
                            }
                        }
                    }
                }
            }
            wl_shm_pool::Request::Destroy => {
                state.shm_pools.remove(&pool_id);
            }
            _ => {}
        }
    }

    fn destroyed(
        state: &mut Self,
        _client: wayland_server::backend::ClientId,
        resource: &wl_shm_pool::WlShmPool,
        _data: &(),
    ) {
        state.shm_pools.remove(&resource.id());
    }
}
