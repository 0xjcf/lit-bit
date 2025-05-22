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
    let max_nodes_for_computation_val =
        proc_macro2::Literal::usize_unsuffixed(builder.all_states.len() * 4); // M * 4

    // --- Generate match-based send method ---
    let mut match_arms = Vec::new();
    for tmp_state in &builder.all_states {
        let from_state_variant = generated_ids
            .full_path_to_variant_ident
            .get(&tmp_state.full_path_name)
            .expect("State variant not found");
        for trans in &tmp_state.transitions {
            let event_pat = trans.event_pattern;
            let target_state_idx = trans
                .target_state_idx
                .expect("Transition target idx not resolved");
            let target_tmp_state = &builder.all_states[target_state_idx];
            let to_state_variant = generated_ids
                .full_path_to_variant_ident
                .get(&target_tmp_state.full_path_name)
                .expect("Target state variant not found");
            let guard = trans.guard_handler;
            let action = trans.action_handler;
            // Build the match arm
            let guard_check = if let Some(guard_expr) = guard {
                quote! {
                    lit_bit_core::core::trace!("[GUARD] Checking guard for {:?} → {:?} on {:?}", #from_state_variant, #to_state_variant, event);
                    if !(#guard_expr(&self.context, event)) {
                        lit_bit_core::core::trace!("[GUARD FAIL] {:?} → {:?} on {:?} blocked by guard", #from_state_variant, #to_state_variant, event);
                        return lit_bit_core::core::SendResult::NoMatch;
                    }
                }
            } else {
                quote! {}
            };
            let action_call = if let Some(action_expr) = action {
                quote! {
                    lit_bit_core::core::trace!("[ACTION] Running action for {:?} → {:?} on {:?}", #from_state_variant, #to_state_variant, event);
                    (#action_expr)(&mut self.context, event);
                }
            } else {
                quote! {}
            };
            match_arms.push(quote! {
                #event_pat => {
                    lit_bit_core::core::trace!("[EVENT] {:?} received in state {:?}", event, #from_state_variant);
                    lit_bit_core::core::trace!("[MATCH] From {:?} on {:?} → {:?}", #from_state_variant, event, #to_state_variant);
                    #guard_check
                    #action_call
                    lit_bit_core::core::trace!("[TRANSITION] {:?} → {:?} via {:?}", #from_state_variant, #to_state_variant, event);
                    self.runtime.transition_to(#state_id_enum_name::#to_state_variant);
                    lit_bit_core::core::trace!("[STATE] Now in state {:?}", #to_state_variant);
                    return lit_bit_core::core::SendResult::Transitioned;
                }
            });
        }
    }
    // Fallback arm
    match_arms.push(quote! { _ => lit_bit_core::core::SendResult::NoMatch });

    let inherent_send_method_body = quote! {
        pub fn send(&mut self, event: &#event_type_path) -> lit_bit_core::core::SendResult {
            match event {
                #(#match_arms),*
            }
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
            pub fn new(context: #context_type_path, initial_event: &#event_type_path) -> Self {
                Self {
                    runtime: lit_bit_core::core::Runtime::new(&#machine_definition_const_ident, context, initial_event),
                }
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
            fn send(&mut self, event: &Self::Event) -> lit_bit_core::core::SendResult {
                #machine_name::send(self, event)
            }
            fn state(&self) -> heapless::Vec<Self::State, {lit_bit_core::core::MAX_ACTIVE_REGIONS}> {
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
            |p_expr| quote! { Some(#p_expr as lit_bit_core::core::EntryExitActionFn<#context_type_path, #event_type_path>) },
        );
        let exit_action_expr = tmp_state.exit_handler.map_or_else(
            || quote! { None },
            |p_expr| quote! { Some(#p_expr as lit_bit_core::core::EntryExitActionFn<#context_type_path, #event_type_path>) },
        );

        let is_parallel_literal = tmp_state.is_parallel;

        state_node_initializers.push(quote! {
            lit_bit_core::core::StateNode {
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
        const STATES: &[lit_bit_core::core::StateNode<#state_id_enum_name, #context_type_path, #event_type_path>] = &[
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
) -> SynResult<TokenStream> {
    let state_id_enum_name = &generated_ids.state_id_enum_name;
    let mut transition_initializers = Vec::new();

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

            // Event variant needs to be constructed based on event_pattern.
            // This is tricky as event_pattern is syn::Pat, not just an Ident.
            // For now, we assume simple `EventEnum::Variant` patterns.
            // A more robust solution would analyze `syn::Pat` to extract the event discriminant.
            // This part is NON-TRIVIAL and was the reason for the old generate_send_method approach.
            // The Runtime must handle matching event: &EventType against pattern: &Pat
            // The Transition struct in core takes `event: EventType`, not a pattern.
            // This implies `Transition.event` field needs re-thinking or `Runtime` needs `Pat` matching.

            // For now, let's use a temporary measure: if pattern is simple Ident, use it.
            // This is INCOMPLETE for pattern matching like `Event::MyEvent { .. }`
            let Some(event_expr) =
                crate::extract_event_pattern_path_tokens(trans_def_ast.event_pattern)
            else {
                return Err(SynError::new(
                    trans_def_ast.event_pattern.span(),
                    "TRANSITIONS array does not yet support this event pattern type. Use simple variant names or struct/tuple variant patterns like 'MyEvent', 'MyEvent::Nested::Variant', 'MyEvent { .. }', or 'MyEvent(..)' for now."
                ));
            };

            let action_expr = trans_def_ast.action_handler.map_or_else(
                || quote! { None },
                |handler_expr| quote! { Some(#handler_expr as lit_bit_core::core::ActionFn<#context_type_path, #event_type_path>) },
            );
            let guard_expr = trans_def_ast.guard_handler.map_or_else(
                || quote! { None },
                |handler_expr| quote! { Some(#handler_expr as lit_bit_core::core::GuardFn<#context_type_path, #event_type_path>) },
            );

            transition_initializers.push(quote! {
                lit_bit_core::core::Transition {
                    from_state: #state_id_enum_name::#from_state_variant,
                    event: #event_expr,
                    to_state: #state_id_enum_name::#to_state_variant,
                    action: #action_expr,
                    guard: #guard_expr,
                }
            });
        }
    }

    let transitions_array_ts = quote! {
        const TRANSITIONS: &[lit_bit_core::core::Transition<
            #state_id_enum_name, #event_type_path, #context_type_path
        >] = &[
            #(#transition_initializers),*
        ];
    };
    Ok(transitions_array_ts)
}
