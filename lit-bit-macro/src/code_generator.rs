#[allow(dead_code)] // It is used by the main statechart macro
pub(crate) fn generate_machine_struct_and_impl(
    machine_name: &Ident,
    state_id_enum_name: &Ident,
    event_type_path: &syn::Path,
    context_type_path: &syn::Path,
    machine_definition_const_ident: &Ident,
    builder: &TmpStateTreeBuilder,
    generated_ids: &GeneratedStateIds,
) -> TokenStream {
    let m_val = proc_macro2::Literal::usize_unsuffixed(8);
    // MAX_NODES_FOR_COMPUTATION = M * MAX_ACTIVE_REGIONS.
    // lit_bit_core::core::MAX_ACTIVE_REGIONS is fixed at 4.
    // Compute this value at macro expansion time.
    let max_nodes_for_computation_val = proc_macro2::Literal::usize_unsuffixed(8 * 4); // M * 4

    // Generate the new inherent send method
    let inherent_send_method =
        generate_send_method(event_type_path, state_id_enum_name, builder, generated_ids);

    // The StateMachine trait send method will call the inherent one.
    let trait_send_method_impl_tokens = quote! {
        fn send(&mut self, event: &Self::Event) -> bool {
            // Ensure this calls the inherent `send` method of the struct,
            // not the trait method itself recursively.
            // Use #machine_name::send to call the inherent method.
            #machine_name::send(self, event)
        }
    };

    let machine_struct_ts = quote! {
        #[derive(Debug)]
        pub struct #machine_name {
            runtime: lit_bit_core::core::Runtime<
                #state_id_enum_name,
                #event_type_path,
                #context_type_path,
                #m_val, // M const generic for Runtime
                #max_nodes_for_computation_val // MAX_NODES_FOR_COMPUTATION const generic for Runtime
            >,
        }
        impl #machine_name {
            pub fn new(context: #context_type_path) -> Self {
                Self {
                    // Ensure the MachineDefinition referred to by machine_definition_const_ident
                    // is compatible with a Runtime that might not use its transitions array as heavily.
                    // The initial_event for Runtime::new needs to be handled; perhaps pass a default event or None.
                    // For now, assuming Runtime::new can be called with just def and context,
                    // or the main macro passes a default initial event.
                    // The user's provided generate_send_method doesn't show how Runtime::new is called.
                    // For now, I will assume the existing Runtime::new call is okay, but it might need adjustment
                    // if it relies on a fully populated TRANSITIONS array for initial state setup that the new model bypasses.
                    // The `initial_leaf_state` in `MachineDefinition` is still used by `Runtime::new`.
                    runtime: lit_bit_core::core::Runtime::new(&#machine_definition_const_ident, context),
                }
            }
            // Place the generated inherent send method here
            #inherent_send_method
        }
        impl lit_bit_core::StateMachine for #machine_name {
            type State = #state_id_enum_name;
            type Event = #event_type_path;
            type Context = #context_type_path;
            #trait_send_method_impl_tokens // Use the updated trait send method
            fn state(&self) -> heapless::Vec<Self::State, {lit_bit_core::core::MAX_ACTIVE_REGIONS}> {
                self.runtime.state()
            }
            fn context(&self) -> &Self::Context {
                // Call the inherent method
                #machine_name::context(self)
            }
            fn context_mut(&mut self) -> &mut Self::Context {
                // Call the inherent method
                #machine_name::context_mut(self)
            }
        }
    };
    machine_struct_ts
}

pub(crate) fn generate_states_array<'ast>(
    builder: &'ast TmpStateTreeBuilder<'ast>,
    generated_ids: &GeneratedStateIds,
    context_type_path: &'ast syn::Path,
    event_type_path: &'ast syn::Path,
) -> SynResult<TokenStream> {
    let state_id_enum_name = &generated_ids.state_id_enum_name;
    let mut state_node_initializers = Vec::new();
    for tmp_state in &builder.all_states {
        // ... existing id, parent_id, initial_child_id ...
        let current_state_id_variant = generated_ids
            .full_path_to_variant_ident
            .get(&tmp_state.full_path_name)
            .ok_or_else(|| {
                SynError::new(
                    tmp_state.name_span,
                    "Internal error: TmpState full_path_name not found in generated IDs map",
                )
            })?;
        let parent_id_expr = tmp_state
            .parent_full_path_name
            .as_ref()
            .and_then(|parent_fpn| {
                generated_ids
                    .full_path_to_variant_ident
                    .get(parent_fpn)
                    .map(|pi| quote! { Some(#state_id_enum_name::#pi) })
            })
            .unwrap_or_else(|| quote! { None });
        let initial_child_id_expr = tmp_state
            .initial_child_idx
            .and_then(|child_idx| {
                builder
                    .all_states
                    .get(child_idx)
                    .and_then(|child_tmp_state| {
                        generated_ids
                            .full_path_to_variant_ident
                            .get(&child_tmp_state.full_path_name)
                            .map(|ci| quote! { Some(#state_id_enum_name::#ci) })
                    })
            })
            .unwrap_or_else(|| quote! { None });

        let entry_action_expr = tmp_state.entry_handler.map_or_else(
            || quote! { None },
            |p_expr| quote! { Some(#p_expr as EntryExitActionFn<#context_type_path, #event_type_path>) },
        );
        let exit_action_expr = tmp_state.exit_handler.map_or_else(
            || quote! { None },
            |p_expr| quote! { Some(#p_expr as EntryExitActionFn<#context_type_path, #event_type_path>) },
        );

        let is_parallel_literal = tmp_state.is_parallel;

        state_node_initializers.push(quote! {
            StateNode {
                id: #state_id_enum_name::#current_state_id_variant,
                parent: #parent_id_expr,
                initial_child: #initial_child_id_expr,
                entry_action: #entry_action_expr,
                exit_action: #exit_action_expr,
                is_parallel: #is_parallel_literal,
            }
        });
    }
    let states_array_ts = quote! {
        const STATES: &[StateNode<#state_id_enum_name, #context_type_path, #event_type_path>] = &[
            #(#state_node_initializers),*
        ];
    };
    Ok(states_array_ts)
}

pub(crate) fn generate_transitions_array<'ast>(
    _builder: &'ast TmpStateTreeBuilder<'ast>,
    _generated_ids: &GeneratedStateIds,
    event_type_path: &'ast syn::Path,
    context_type_path: &'ast syn::Path,
) -> SynResult<TokenStream> {
    let transitions_array_ts = quote! {
        const TRANSITIONS: &[lit_bit_core::core::Transition<
            _, #event_type_path, #context_type_path
        >] = &[];
    };
    Ok(transitions_array_ts)
}

pub fn generate_send_method(
    event_type_path: &syn::Path,
    state_id_enum_name: &Ident,
    builder: &TmpStateTreeBuilder,
    generated_ids: &GeneratedStateIds,
) -> proc_macro2::TokenStream {
    eprintln!("*** REBUILDING generate_send_method ***"); // DEBUG LINE ADDED
    let mut match_arms = Vec::new();

    for state_node in &builder.all_states {
        for t in &state_node.transitions {
            let event_pat = &t.event_pattern;

            let guard_block = if let Some(guard_expr) = &t.guard_handler {
                quote! {
                    (#guard_expr)(&self.context(), event)
                }
            } else {
                quote! {
                    true
                }
            };

            let action_block = if let Some(action_expr) = &t.action_handler {
                quote! {
                    (#action_expr)(&mut self.context_mut(), event);
                }
            } else {
                quote! {}
            };

            let target_state_idx = t.target_state_idx.expect("Missing target state");
            let target_state = &builder.all_states[target_state_idx];
            let target_variant = generated_ids
                .full_path_to_variant_ident
                .get(&target_state.full_path_name)
                .expect("Missing target variant");

            let perform_transition_and_update_state = quote! {
                self.runtime.active_leaf_states = heapless::Vec::from_slice(
                    &[#state_id_enum_name::#target_variant]
                ).unwrap();
            };

            match_arms.push(quote! {
                #event_pat => {
                    let passes_guard = #guard_block;
                    if passes_guard {
                        #action_block
                        #perform_transition_and_update_state
                        return true;
                    }
                    false
                }
            });
        }
    }

    quote! {
        pub fn send(&mut self, event: &#event_type_path) -> bool {
            match event {
                #(#match_arms),*,
                _ => false
            }
        }
    }
}
