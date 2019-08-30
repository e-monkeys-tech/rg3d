use crate::{
    scene::{Scene, node::Node},
    utils::pool::Handle,
    engine::State,
    resource::{
        fbx,
        Resource,
        ResourceKind,
        fbx::error::FbxError,
    },
    scene::node::NodeKind,
};
use std::{
    path::Path,
    cell::RefCell,
    rc::Rc,
    collections::{HashMap, hash_map::Entry},
};

pub struct Model {
    scene: Scene,
}

impl Default for Model {
    fn default() -> Self {
        Self {
            scene: Scene::new(),
        }
    }
}

impl Model {
    pub fn load(path: &Path, state: &mut State) -> Result<Model, FbxError> {
        let mut scene = Scene::new();
        fbx::load_to_scene(&mut scene, state, path)?;
        Ok(Model { scene })
    }

    /// Tries to instantiate model from given resource. Returns non-none handle on success.
    pub fn instantiate(resource_rc: Rc<RefCell<Resource>>, dest_scene: &mut Scene) -> Result<Handle<Node>, ()> {
        let resource = resource_rc.borrow();
        if let ResourceKind::Model(model) = resource.borrow_kind() {
            let mut old_new_mapping = HashMap::new();
            let root = model.scene.copy_node(model.scene.get_root(), dest_scene, &mut old_new_mapping);

            // Notify instantiated nodes about resource they were created from. Also do bones
            // remapping for meshes.
            let mut stack = Vec::new();
            stack.push(root);
            while let Some(node_handle) = stack.pop() {
                if let Some(node) = dest_scene.get_nodes_mut().borrow_mut(node_handle) {
                    node.set_resource(Rc::clone(&resource_rc));

                    // Remap bones
                    if let NodeKind::Mesh(mesh) = node.borrow_kind_mut() {
                        for surface in mesh.get_surfaces_mut() {
                            for bone_handle in surface.bones.iter_mut() {
                                if let Entry::Occupied(entry) = old_new_mapping.entry(bone_handle.clone()) {
                                    *bone_handle = *entry.get();
                                }
                            }
                        }
                    }

                    // Continue on children.
                    for child_handle in node.get_children() {
                        stack.push(child_handle.clone());
                    }
                }
            }

            // Instantiate animations
            for ref_anim in model.scene.get_animations().iter() {
                let mut anim_copy = ref_anim.clone();

                // Remap animation track nodes.
                for (i, ref_track) in ref_anim.get_tracks().iter().enumerate() {
                    // Find instantiated node that corresponds to node in resource
                    let nodes = dest_scene.get_nodes();
                    for k in 0..nodes.get_capacity() {
                        if let Some(node) = nodes.at(k) {
                            if node.get_original_handle() == ref_track.get_node() {
                                anim_copy.get_tracks_mut()[i].set_node(nodes.handle_from_index(k));
                            }
                        }
                    }
                }

                dest_scene.add_animation(anim_copy);
            }

            return Ok(root);
        }
        Err(())
    }

    pub fn get_scene_mut(&mut self) -> &mut Scene {
        &mut self.scene
    }

    pub fn get_scene(&self) -> &Scene {
        &self.scene
    }

    pub fn find_node_by_name(&self, name: &str) -> Handle<Node> {
        self.scene.find_node_by_name(self.scene.get_root(), name)
    }
}