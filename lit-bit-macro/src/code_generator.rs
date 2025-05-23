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
    let m_val = proc_macro2::Literal::usize_unsuffixed(builder.all_states.len());
    let max_nodes_for_computation_val = proc_macro2::Literal::usize_unsuffixed(
        builder.all_states.len() * lit_bit_core::MAX_ACTIVE_REGIONS,
    );

    // --- Remove problematic match generation - delegate to Runtime instead ---
    let inherent_send_method_body = quote! {
        pub fn send(&mut self, event: &#event_type_path) -> lit_bit_core::SendResult {
            use lit_bit_core::StateMachine;
            self.runtime.send(event)
        }
    };

    let machine_struct_ts = quote! {
        #[derive(Debug)]
        pub struct #machine_name {
            runtime: lit_bit_core::Runtime<
                #state_id_enum_name,
                #event_type_path,
                #context_type_path,
                #m_val, // M const generic for Runtime
                #max_nodes_for_computation_val // MAX_NODES_FOR_COMPUTATION const generic for Runtime
            >,
        }
        impl #machine_name {
            pub fn new(context: #context_type_path, initial_event: &#event_type_path) -> Result<Self, lit_bit_core::ProcessingError> {
                let runtime = lit_bit_core::Runtime::new(&#machine_definition_const_ident, context, initial_event)?;
                Ok(Self { runtime })
            }
            #inherent_send_method_body
            pub fn context(&self) -> &#context_type_path {
                self.runtime.context()
            }
            pub fn context_mut(&mut self) -> &mut #context_type_path {
                self.runtime.context_mut()
            }
        }
        impl lit_bit_core::StateMachine for #machine_name {
            type State = #state_id_enum_name;
            type Event = #event_type_path;
            type Context = #context_type_path;
            fn send(&mut self, event: &Self::Event) -> lit_bit_core::SendResult {
                #machine_name::send(self, event)
            }
            fn state(&self) -> heapless::Vec<Self::State, {lit_bit_core::MAX_ACTIVE_REGIONS}> {
                self.runtime.state()
            }
            fn context(&self) -> &Self::Context {
                self.runtime.context()
            }
            fn context_mut(&mut self) -> &mut Self::Context {
                self.runtime.context_mut()
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
            |p_expr| quote! { Some(#p_expr as lit_bit_core::EntryExitActionFn<#context_type_path, #event_type_path>) },
        );
        let exit_action_expr = tmp_state.exit_handler.map_or_else(
            || quote! { None },
            |p_expr| quote! { Some(#p_expr as lit_bit_core::EntryExitActionFn<#context_type_path, #event_type_path>) },
        );

        let is_parallel_literal = tmp_state.is_parallel;

        state_node_initializers.push(quote! {
            lit_bit_core::StateNode {
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
        const STATES: &[lit_bit_core::StateNode<#state_id_enum_name, #context_type_path, #event_type_path>] = &[
            #(#state_node_initializers),*
        ];
    };
    Ok(states_array_ts)
}

pub(crate) fn generate_transitions_array<'ast>(
    builder: &'ast TmpStateTreeBuilder<'ast>,
    generated_ids: &GeneratedStateIds,
    event_type_path: &'ast syn::Path,
    context_type_path: &'ast syn::Path,
    machine_name: &Ident,
) -> SynResult<TokenStream> {
    let state_id_enum_name = &generated_ids.state_id_enum_name;
    let mut transition_initializers = Vec::new();
    let mut matcher_fns = Vec::new();

    for tmp_state_node in &builder.all_states {
        let from_state_variant = generated_ids
            .full_path_to_variant_ident
            .get(&tmp_state_node.full_path_name)
            .ok_or_else(|| {
                SynError::new(
                    tmp_state_node.name_span,
                    "Internal error: TmpState full_path_name not found in generated IDs map for 'from_state'",
                )
            })?;

        for trans_def_ast in &tmp_state_node.transitions {
            let target_state_idx = trans_def_ast.target_state_idx.ok_or_else(|| {
                SynError::new(
                    trans_def_ast.target_state_path_ast.span(),
                    "Internal error: Transition target_state_idx not resolved",
                )
            })?;
            let target_tmp_state = builder.all_states.get(target_state_idx).ok_or_else(|| {
                SynError::new(
                    trans_def_ast.target_state_path_ast.span(),
                    "Internal error: Transition target_state_idx refers to out-of-bounds state",
                )
            })?;
            let to_state_variant = generated_ids
                .full_path_to_variant_ident
                .get(&target_tmp_state.full_path_name)
                .ok_or_else(|| {
                    SynError::new(
                        target_tmp_state.name_span,
                        "Internal error: TmpState full_path_name not found for 'to_state'",
                    )
                })?;

            let event_pattern = trans_def_ast.event_pattern;
            let event_pattern_tokens = quote! { #event_pattern };

            // Generate a unique matcher function ident for each transition
            let matcher_fn_ident = quote::format_ident!(
                "matches_{}_T{}",
                machine_name,
                transition_initializers.len()
            );
            let matcher_fn = quote! {
                fn #matcher_fn_ident(e: &#event_type_path) -> bool {
                    matches!(e, &#event_pattern_tokens)
                }
            };
            matcher_fns.push(matcher_fn);

            let action_expr = trans_def_ast.action_handler.map_or_else(
                || quote! { None },
                |handler_expr| quote! { Some(#handler_expr as lit_bit_core::ActionFn<#context_type_path, #event_type_path>) },
            );
            let guard_expr = trans_def_ast.guard_handler.map_or_else(
                || quote! { None },
                |handler_expr| quote! { Some(#handler_expr as lit_bit_core::GuardFn<#context_type_path, #event_type_path>) },
            );

            transition_initializers.push(quote! {
                lit_bit_core::Transition {
                    from_state: #state_id_enum_name::#from_state_variant,
                    to_state: #state_id_enum_name::#to_state_variant,
                    action: #action_expr,
                    guard: #guard_expr,
                    match_fn: Some(#matcher_fn_ident),
                }
            });
        }
    }

    let transitions_array_ts = quote! {
        #(#matcher_fns)*
        const TRANSITIONS: &[lit_bit_core::Transition<
            #state_id_enum_name, #event_type_path, #context_type_path
        >] = &[
            #(#transition_initializers),*
        ];
    };
    Ok(transitions_array_ts)
}
