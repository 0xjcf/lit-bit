use proc_macro::TokenStream;
use quote::quote;
use syn::{
    braced, bracketed,
    parse::{Parse, ParseStream},
    Ident, Path, Result, Token,
};

// Define keywords for parsing
mod keywords {
    syn::custom_keyword!(name);
    syn::custom_keyword!(context);
    syn::custom_keyword!(event);
    syn::custom_keyword!(initial);
    syn::custom_keyword!(state);
    syn::custom_keyword!(on);
    syn::custom_keyword!(entry);
    syn::custom_keyword!(exit);
    syn::custom_keyword!(action);
    syn::custom_keyword!(guard);
    syn::custom_keyword!(parallel); // New
}

// Define attribute structures BEFORE StateDeclarationAst
#[derive(Debug, Clone, PartialEq)]
enum StateAttributeAst {
    Parallel(keywords::parallel),
}

impl Parse for StateAttributeAst {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(keywords::parallel) {
            Ok(StateAttributeAst::Parallel(input.parse()?))
        } else {
            Err(input.error("Expected 'parallel' attribute within state attribute brackets"))
        }
    }
}

#[derive(Debug)]
struct StateAttributesInputAst {
    #[allow(dead_code)]
    bracket_token: syn::token::Bracket,
    attributes: syn::punctuated::Punctuated<StateAttributeAst, Token![,]>,
}

impl Parse for StateAttributesInputAst {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        let bracket_token = bracketed!(content in input);
        let attributes: syn::punctuated::Punctuated<StateAttributeAst, Token![,]> =
            content.parse_terminated(StateAttributeAst::parse, Token![,])?;

        if attributes.is_empty() {
            return Err(syn::Error::new(bracket_token.span.join(), "State attribute list cannot be empty if brackets are present. Expected at least one attribute like '[parallel]'."));
        }

        Ok(StateAttributesInputAst {
            bracket_token,
            attributes,
        })
    }
}

// Overall structure for the statechart! macro input
#[derive(Debug)]
#[allow(dead_code)]
struct StateChartInputAst {
    name_keyword_token: keywords::name,
    name: Ident,
    comma1: Token![,],
    context_keyword_token: keywords::context,
    context_type: Path,
    comma2: Token![,],
    event_keyword_token: keywords::event,
    event_type: Path,
    comma3: Token![,],
    initial_keyword_token: keywords::initial,
    initial_target_expression: Path,
    comma4: Option<Token![,]>,
    top_level_states: Vec<StateDeclarationAst>,
}

impl Parse for StateChartInputAst {
    fn parse(input: ParseStream) -> Result<Self> {
        let name_keyword_token: keywords::name = input.parse()?;
        input.parse::<Token![:]>()?;
        let name: Ident = input.parse()?;
        let comma1: Token![,] = input.parse()?;

        let context_keyword_token: keywords::context = input.parse()?;
        input.parse::<Token![:]>()?;
        let context_type: Path = input.parse()?;
        let comma2: Token![,] = input.parse()?;

        let event_keyword_token: keywords::event = input.parse()?;
        input.parse::<Token![:]>()?;
        let event_type: Path = input.parse()?;
        let comma3: Token![,] = input.parse()?;

        // Try parsing as Ident and verify
        let initial_ident: Ident = input.parse()?;
        if initial_ident != "initial" {
            return Err(syn::Error::new(
                initial_ident.span(),
                format!("Expected 'initial' keyword, got '{initial_ident}'"),
            ));
        }
        let initial_keyword_token_span = initial_ident.span();

        input.parse::<Token![:]>()?;
        let initial_target_expression: Path = input.parse()?;

        let comma4: Option<Token![,]> = if input.peek(Token![,]) {
            Some(input.parse()?)
        } else {
            None
        };

        let mut top_level_states = Vec::new();
        while input.peek(keywords::state) {
            top_level_states.push(input.parse()?);
        }

        if !input.is_empty() && comma4.is_none() && !input.peek(keywords::state) {
            return Err(input.error("Expected 'state' keyword or end of input after header"));
        }

        Ok(StateChartInputAst {
            name_keyword_token,
            name,
            comma1,
            context_keyword_token,
            context_type,
            comma2,
            event_keyword_token,
            event_type,
            comma3,
            initial_keyword_token: keywords::initial {
                span: initial_keyword_token_span,
            },
            initial_target_expression,
            comma4,
            top_level_states,
        })
    }
}

#[derive(Debug)]
#[allow(dead_code)]
struct StateDeclarationAst {
    state_keyword_token: keywords::state,
    name: Ident,
    attributes: Option<StateAttributesInputAst>, // New field
    brace_token: syn::token::Brace,
    default_child_declaration: Option<DefaultChildDeclarationAst>,
    body_items: Vec<StateBodyItemAst>,
}

impl Parse for StateDeclarationAst {
    fn parse(input: ParseStream) -> Result<Self> {
        let state_keyword_token: keywords::state = input.parse()?;
        let name: Ident = input.parse()?;

        let attributes: Option<StateAttributesInputAst> = if input.peek(syn::token::Bracket) {
            Some(input.parse()?)
        } else {
            None
        };

        let content_in_braces;
        let brace_token = braced!(content_in_braces in input);

        let default_child_declaration: Option<DefaultChildDeclarationAst> =
            if content_in_braces.peek(keywords::initial) {
                Some(content_in_braces.parse()?)
            } else {
                None
            };

        let mut body_items = Vec::new();
        while !content_in_braces.is_empty() {
            if content_in_braces.peek(keywords::entry) {
                body_items.push(StateBodyItemAst::EntryHook(content_in_braces.parse()?));
            } else if content_in_braces.peek(keywords::exit) {
                body_items.push(StateBodyItemAst::ExitHook(content_in_braces.parse()?));
            } else if content_in_braces.peek(keywords::on) {
                // When creating StateBodyItemAst::Transition, wrap the parsed TransitionDefinitionAst in a Box
                body_items.push(StateBodyItemAst::Transition(Box::new(
                    content_in_braces.parse()?,
                )));
            } else if content_in_braces.peek(keywords::state) {
                body_items.push(StateBodyItemAst::NestedState(Box::new(
                    content_in_braces.parse()?,
                )));
            } else {
                return Err(content_in_braces.error("Unexpected token inside state block. Expected 'initial', 'entry', 'exit', 'on', or nested 'state'."));
            }
        }

        Ok(StateDeclarationAst {
            state_keyword_token,
            name,
            attributes,
            brace_token,
            default_child_declaration,
            body_items,
        })
    }
}

#[derive(Debug)]
#[allow(dead_code)]
struct DefaultChildDeclarationAst {
    initial_keyword_token: keywords::initial,
    colon_token: Token![:],
    child_state_expression: Path,
    semi_token: Token![;],
}

impl Parse for DefaultChildDeclarationAst {
    fn parse(input: ParseStream) -> Result<Self> {
        let initial_keyword_token: keywords::initial = input.parse()?;
        let colon_token: Token![:] = input.parse()?;
        let child_state_expression: Path = input.parse()?;
        let semi_token: Token![;] = input.parse()?;
        Ok(DefaultChildDeclarationAst {
            initial_keyword_token,
            colon_token,
            child_state_expression,
            semi_token,
        })
    }
}

#[derive(Debug)]
#[allow(dead_code)]
enum StateBodyItemAst {
    EntryHook(LifecycleHookAst),
    ExitHook(LifecycleHookAst),
    Transition(Box<TransitionDefinitionAst>), // Boxed this variant
    NestedState(Box<StateDeclarationAst>),
}

#[derive(Debug)]
#[allow(dead_code)]
struct LifecycleHookAst {
    kind: Ident,
    colon_token: Token![:],
    hook_function_expression: syn::Expr, // Changed from Path
    semi_token: Token![;],
}

impl Parse for LifecycleHookAst {
    fn parse(input: ParseStream) -> Result<Self> {
        let kind: Ident = input.parse()?;
        if kind != "entry" && kind != "exit" {
            return Err(syn::Error::new(
                kind.span(),
                "Expected 'entry' or 'exit' keyword for lifecycle hook",
            ));
        }
        let colon_token: Token![:] = input.parse()?;

        // Removed dot_token check as syn::Expr handles .foo and self.foo correctly.
        // if input.peek(Token![.]) {
        //     let dot_token: Token![.] = input.parse()?;
        //     let _member: Ident = input.parse()?;
        //     return Err(syn::Error::new(dot_token.span, "Leading dot notation for hooks (e.g., `.foo`) is not yet fully supported. Use `self.foo` or a full path."));
        // }

        let hook_function_expression: syn::Expr = input.parse()?; // Changed from Path
        let semi_token: Token![;] = input.parse()?;

        Ok(LifecycleHookAst {
            kind,
            colon_token,
            hook_function_expression,
            semi_token,
        })
    }
}

#[derive(Debug)]
#[allow(dead_code)]
struct TransitionDefinitionAst {
    on_keyword_token: keywords::on,
    event_name: Ident,
    guard_clause: Option<GuardConditionAst>,
    arrow_token: Token![=>],
    target_state_path: Path,
    action_clause: Option<TransitionActionAst>,
    semi_token: Token![;],
}

impl Parse for TransitionDefinitionAst {
    fn parse(input: ParseStream) -> Result<Self> {
        let on_keyword_token: keywords::on = input.parse()?;
        let event_name: Ident = input.parse()?;

        let guard_clause: Option<GuardConditionAst> = if input.peek(syn::token::Bracket) {
            let fork = input.fork();
            let content_in_brackets_for_guard;
            syn::bracketed!(content_in_brackets_for_guard in fork);
            if content_in_brackets_for_guard.peek(keywords::guard) {
                Some(input.parse()?)
            } else {
                None
            }
        } else {
            None
        };

        let arrow_token: Token![=>] = input.parse()?;
        let target_state_path: Path = input.parse()?;

        let action_clause: Option<TransitionActionAst> = if input.peek(syn::token::Bracket) {
            let fork = input.fork();
            let content_in_brackets_for_action;
            syn::bracketed!(content_in_brackets_for_action in fork);

            if content_in_brackets_for_action.peek(keywords::action)
                || content_in_brackets_for_action.peek(Ident)
            {
                Some(input.parse()?)
            } else if content_in_brackets_for_action.peek(Token![.]) {
                let content_to_error_on;
                let _bracket_token_for_error = syn::bracketed!(content_to_error_on in input);
                let dot_token: Token![.] = content_to_error_on.parse()?;
                return Err(syn::Error::new(dot_token.span, "Leading dot notation for action handlers (e.g., `[.foo]`) is not yet supported. Use `[self.foo]` or `[path::to::foo]`."));
            } else {
                None
            }
        } else {
            None
        };

        let semi_token: Token![;] = input.parse()?;

        Ok(TransitionDefinitionAst {
            on_keyword_token,
            event_name,
            guard_clause,
            arrow_token,
            target_state_path,
            action_clause,
            semi_token,
        })
    }
}

#[derive(Debug)]
#[allow(dead_code)]
struct GuardConditionAst {
    bracket_token: syn::token::Bracket,
    guard_keyword_token: keywords::guard,
    condition_function_expression: syn::Expr, // Changed from Path
}

impl Parse for GuardConditionAst {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        let bracket_token = bracketed!(content in input);
        let guard_keyword_token: keywords::guard = content.parse()?;
        let condition_function_expression: syn::Expr = content.parse()?; // Changed from Path
        if !content.is_empty() {
            return Err(
                content.error("Unexpected tokens after guard condition expression inside brackets")
            );
        }
        Ok(GuardConditionAst {
            bracket_token,
            guard_keyword_token,
            condition_function_expression,
        })
    }
}

#[derive(Debug)]
#[allow(dead_code)]
struct TransitionActionAst {
    bracket_token: syn::token::Bracket,
    action_keyword_token: Option<keywords::action>,
    transition_action_expression: syn::Expr, // Changed from Path
}

impl Parse for TransitionActionAst {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        let bracket_token = bracketed!(content in input);
        let action_keyword_token = if content.peek(keywords::action) {
            Some(content.parse()?)
        } else {
            None
        };
        let transition_action_expression: syn::Expr = content.parse()?; // Changed from Path
        if !content.is_empty() {
            return Err(content
                .error("Unexpected tokens after transition action expression inside brackets"));
        }
        Ok(TransitionActionAst {
            bracket_token,
            action_keyword_token,
            transition_action_expression,
        })
    }
}

// --- Stage 2: Semantic Analysis & Intermediate Representation ---

// This module will contain the logic for building a temporary tree representation
// from the AST, validating it, and then preparing it for flattening.

// Using a new module scope for these temporary structures and builder logic.
pub(crate) mod intermediate_tree {
    use proc_macro2::Span;
    use quote::ToTokens;
    use std::collections::{HashMap, HashSet};
    use syn::spanned::Spanned;
    use syn::{Error as SynError, Expr, Ident, Path, Result as SynResult}; // Ensure Expr is imported // Keep for target_path_ast.to_token_stream()

    #[derive(Debug, Clone)]
    #[allow(dead_code)]
    pub(crate) struct TmpTransition<'ast> {
        pub event_name: &'ast Ident,
        pub target_state_path_ast: &'ast Path,
        pub target_state_idx: Option<usize>,
        pub guard_handler: Option<&'ast Expr>, // Changed from Path
        pub action_handler: Option<&'ast Expr>, // Changed from Path
        pub on_keyword_span: Span,
    }

    #[derive(Debug)]
    #[allow(dead_code)]
    pub(crate) struct TmpState<'ast> {
        pub local_name: &'ast Ident,
        pub full_path_name: String,
        pub parent_full_path_name: Option<String>,
        pub depth: usize,
        pub children_indices: Vec<usize>,
        pub initial_child_idx: Option<usize>,
        pub entry_handler: Option<&'ast Expr>, // Changed from Path
        pub exit_handler: Option<&'ast Expr>,  // Changed from Path
        pub transitions: Vec<TmpTransition<'ast>>,
        pub is_parallel: bool, // New field
        pub state_keyword_span: Span,
        pub name_span: Span,
        pub declared_initial_child_expression: Option<&'ast Path>,
    }

    pub(crate) struct TmpStateTreeBuilder<'ast> {
        #[allow(dead_code)]
        pub all_states: Vec<TmpState<'ast>>,
        pub defined_full_paths: HashSet<String>,
        pub state_full_path_to_idx_map: HashMap<String, usize>,
    }

    impl<'ast> TmpStateTreeBuilder<'ast> {
        pub fn new() -> Self {
            Self {
                all_states: Vec::new(),
                defined_full_paths: HashSet::new(),
                state_full_path_to_idx_map: HashMap::new(),
            }
        }

        pub fn build_from_ast(
            &mut self,
            input_ast: &'ast crate::StateChartInputAst,
        ) -> SynResult<()> {
            let mut top_level_names = HashSet::new();
            for state_decl_ast in &input_ast.top_level_states {
                let name_str = state_decl_ast.name.to_string();
                if !top_level_names.insert(name_str.clone()) {
                    return Err(SynError::new(
                        state_decl_ast.name.span(),
                        format!("Duplicate top-level state name defined: {name_str}"),
                    ));
                }
            }

            for state_decl_ast in &input_ast.top_level_states {
                self.process_state_declaration(state_decl_ast, None, 0, &mut HashSet::new())?;
            }

            // Populate the full_path_to_idx_map after all states are discovered
            for (idx, state_node) in self.all_states.iter().enumerate() {
                self.state_full_path_to_idx_map
                    .insert(state_node.full_path_name.clone(), idx);
            }

            // Second pass: Resolve initial children
            self.resolve_and_validate_initial_children()?;

            // Third pass: Resolve transition targets
            self.resolve_and_validate_transition_targets()?;

            // TODO: Further validations (max depth, etc.)
            Ok(())
        }

        fn extract_ident_from_path(path: &'ast Path) -> Option<&'ast Ident> {
            if path.leading_colon.is_none() && path.segments.len() == 1 {
                return Some(&path.segments[0].ident);
            }
            None
        }

        fn resolve_and_validate_initial_children(&mut self) -> SynResult<()> {
            for i in 0..self.all_states.len() {
                let parent_state_full_path = self.all_states[i].full_path_name.clone(); // Keep for existing error messages if needed
                let parent_has_children = !self.all_states[i].children_indices.is_empty();
                let declared_initial_expr_opt =
                    self.all_states[i].declared_initial_child_expression;

                let initial_decl_span = declared_initial_expr_opt
                    .map_or_else(|| self.all_states[i].name_span, Spanned::span);

                let current_state = &self.all_states[i]; // More direct access

                if current_state.is_parallel {
                    // Validation 1: Parallel state must have at least two children (regions)
                    if current_state.children_indices.len() < 2 {
                        return Err(SynError::new(
                            current_state.name_span,
                            format!(
                                "Parallel state '{}' must have at least two child regions.",
                                current_state.full_path_name
                            ),
                        ));
                    }

                    // Validation 2: Parallel state should not have an 'initial:' declaration itself
                    if current_state.declared_initial_child_expression.is_some() {
                        // Use the span of the 'initial:' declaration for the error
                        let error_span = current_state
                            .declared_initial_child_expression
                            .unwrap()
                            .span();
                        return Err(SynError::new(error_span,
                            format!("Parallel state '{}' must not declare an 'initial' child for itself. Initial states are defined within its regions.", current_state.full_path_name)));
                    }

                    // Validation 3: Each direct child (region) of a parallel state, if compound, must declare an initial state.
                    for &child_idx in &current_state.children_indices {
                        let region_state = &self.all_states[child_idx];
                        let region_is_compound_by_having_children =
                            !region_state.children_indices.is_empty();
                        // A region is compound if it HAS children. Its own initial_child_idx being Some also indicates it was declared compound.
                        if region_is_compound_by_having_children
                            && region_state.declared_initial_child_expression.is_none()
                        {
                            return Err(SynError::new(region_state.name_span,
                                format!("Region '{}' within parallel state '{}' is a compound state and must declare an 'initial' child.", region_state.full_path_name, current_state.full_path_name)));
                        }
                    }
                } else {
                    // Not parallel: existing logic for compound states
                    if parent_has_children && declared_initial_expr_opt.is_none() {
                        return Err(SynError::new(
                            self.all_states[i].name_span,
                            format!(
                                "Compound state '{parent_state_full_path}' must declare an 'initial' child state."
                            ),
                        ));
                    } else if !parent_has_children && declared_initial_expr_opt.is_some() {
                        return Err(SynError::new(initial_decl_span,
                            format!("State '{parent_state_full_path}' declares an 'initial' child but has no nested states defined.")));
                    }

                    if let Some(initial_path) = declared_initial_expr_opt {
                        let initial_child_local_ident = Self::extract_ident_from_path(initial_path)
                            .ok_or_else(|| SynError::new(initial_path.span(),
                                "'initial' state target must be a simple identifier (name of a direct child state)."))?;

                        let initial_child_local_name = initial_child_local_ident.to_string();
                        let expected_child_full_path =
                            format!("{parent_state_full_path}_{initial_child_local_name}");

                        let mut found_child_idx: Option<usize> = None;
                        for &child_idx_in_all_states in &self.all_states[i].children_indices {
                            if self.all_states[child_idx_in_all_states].full_path_name
                                == expected_child_full_path
                                && self.all_states[child_idx_in_all_states].local_name
                                    == initial_child_local_ident
                            {
                                found_child_idx = Some(child_idx_in_all_states);
                                break;
                            }
                        }

                        match found_child_idx {
                            Some(idx) => {
                                self.all_states[i].initial_child_idx = Some(idx);
                            }
                            None => {
                                return Err(SynError::new(initial_path.span(),
                                    format!("Initial child '{initial_child_local_name}' declared for state '{parent_state_full_path}' is not defined as a direct child of this state.")));
                            }
                        }
                    }
                }
            }
            Ok(())
        }

        // Made this pub(crate) so code_generator can use it via TmpStateTreeBuilder::path_to_string_for_lookup
        pub(crate) fn path_to_string_for_lookup(path: &Path) -> String {
            path.segments
                .iter()
                .map(|segment| segment.ident.to_string())
                .collect::<Vec<String>>()
                .join("_")
        }

        fn resolve_path_to_state_index(
            &self,
            current_state_idx_for_context: usize,
            target_path_ast: &'ast Path,
        ) -> SynResult<usize> {
            let target_path_span = target_path_ast.span();
            let current_tmp_state = &self.all_states[current_state_idx_for_context];

            if target_path_ast.leading_colon.is_some() {
                return Err(SynError::new(
                    target_path_span,
                    "Absolute paths starting with `::` are not supported for transition targets.",
                ));
            }

            let normalized_target_full_path_candidate =
                Self::path_to_string_for_lookup(target_path_ast);

            if target_path_ast.segments.len() == 1 {
                let target_local_name = target_path_ast.segments[0].ident.to_string();

                let direct_child_full_name =
                    format!("{}_{}", current_tmp_state.full_path_name, target_local_name);
                if let Some(idx) = self.state_full_path_to_idx_map.get(&direct_child_full_name) {
                    if current_tmp_state.children_indices.contains(idx) {
                        return Ok(*idx);
                    }
                }

                // Corrected else if for clippy::collapsible_else_if
                if let Some(parent_full_path) = &current_tmp_state.parent_full_path_name {
                    let sibling_full_name = format!("{parent_full_path}_{target_local_name}");
                    if let Some(idx) = self.state_full_path_to_idx_map.get(&sibling_full_name) {
                        return Ok(*idx);
                    }
                } else if let Some(idx) = self.state_full_path_to_idx_map.get(&target_local_name) {
                    if self.all_states[*idx].parent_full_path_name.is_none() {
                        return Ok(*idx);
                    }
                }

                if let Some(idx) = self.state_full_path_to_idx_map.get(&target_local_name) {
                    return Ok(*idx);
                }
            }

            if let Some(idx) = self
                .state_full_path_to_idx_map
                .get(&normalized_target_full_path_candidate)
            {
                return Ok(*idx);
            }

            // Corrected: Use target_path_ast.to_token_stream() directly for clippy::to_string_in_format_args
            Err(SynError::new(target_path_span, format!("Transition target state '{normalized_target_full_path_candidate}' (normalized from AST path: '{}') not found or path is ambiguous.", target_path_ast.to_token_stream())))
        }

        fn resolve_and_validate_transition_targets(&mut self) -> SynResult<()> {
            for i in 0..self.all_states.len() {
                let transitions_info: Vec<(&'ast Path, Span)> = self.all_states[i]
                    .transitions
                    .iter()
                    .map(|t| (t.target_state_path_ast, t.on_keyword_span))
                    .collect();

                let mut resolved_indices = Vec::new();
                for (target_path_ast, on_span) in transitions_info {
                    match self.resolve_path_to_state_index(i, target_path_ast) {
                        Ok(idx) => resolved_indices.push(Some(idx)),
                        Err(e) => {
                            let final_span = target_path_ast.span().resolved_at(on_span);
                            return Err(SynError::new(final_span, e.to_string()));
                        }
                    }
                }

                let state_transitions = &mut self.all_states[i].transitions;
                for (j, transition) in state_transitions.iter_mut().enumerate() {
                    transition.target_state_idx = resolved_indices[j];
                }
            }
            Ok(())
        }

        #[allow(clippy::too_many_lines)]
        fn process_state_declaration(
            &mut self,
            state_decl_ast: &'ast crate::StateDeclarationAst,
            current_parent_full_path: Option<&str>,
            depth: usize,
            sibling_local_names: &mut HashSet<String>,
        ) -> SynResult<usize> {
            let local_name_str = state_decl_ast.name.to_string();

            if !sibling_local_names.insert(local_name_str.clone()) {
                return Err(SynError::new(
                    state_decl_ast.name.span(),
                    format!("Duplicate state name '{local_name_str}' at this level."),
                ));
            }

            let full_path_name = match current_parent_full_path {
                Some(parent_path) => format!("{parent_path}_{local_name_str}"),
                None => local_name_str.clone(),
            };

            if self.defined_full_paths.contains(&full_path_name) {
                return Err(SynError::new(
                    state_decl_ast.name.span(),
                    format!("State full path '{full_path_name}' is not unique. This can happen with duplicate top-level names or identically named nested states under the same hierarchy."),
                ));
            }
            self.defined_full_paths.insert(full_path_name.clone());

            let mut is_parallel_flag = false;
            if let Some(attrs_input) = &state_decl_ast.attributes {
                for attr in &attrs_input.attributes {
                    match attr {
                        crate::StateAttributeAst::Parallel(_) => {
                            // Fully qualified path
                            if is_parallel_flag {
                                // Optionally error or warn on duplicate [parallel, parallel]
                                // For now, just allow it, effect is idempotent.
                            }
                            is_parallel_flag = true;
                        }
                    }
                }
            }

            let current_node_index = self.all_states.len();
            let new_state_node = TmpState {
                local_name: &state_decl_ast.name,
                full_path_name: full_path_name.clone(),
                parent_full_path_name: current_parent_full_path.map(String::from),
                depth,
                children_indices: Vec::new(),
                initial_child_idx: None, // Will be resolved in a later pass
                entry_handler: None,     // Placeholder
                exit_handler: None,      // Placeholder
                transitions: Vec::new(), // Placeholder
                is_parallel: is_parallel_flag, // Set based on parsed attributes
                state_keyword_span: state_decl_ast.state_keyword_token.span(),
                name_span: state_decl_ast.name.span(),
                declared_initial_child_expression: state_decl_ast
                    .default_child_declaration
                    .as_ref()
                    .map(|dcd| &dcd.child_state_expression),
            };
            self.all_states.push(new_state_node);

            let mut children_indices_for_this_state = Vec::new();
            // Correct types for local handler options
            let mut entry_handler_opt: Option<&'ast Expr> = None; // Changed from Path
            let mut exit_handler_opt: Option<&'ast Expr> = None; // Changed from Path
            let mut transitions_for_this_state: Vec<TmpTransition<'ast>> = Vec::new();

            for item in &state_decl_ast.body_items {
                match item {
                    crate::StateBodyItemAst::EntryHook(hook_ast) => {
                        entry_handler_opt = Some(&hook_ast.hook_function_expression);
                    }
                    crate::StateBodyItemAst::ExitHook(hook_ast) => {
                        exit_handler_opt = Some(&hook_ast.hook_function_expression);
                    }
                    // trans_ast is now &Box<TransitionDefinitionAst> due to pattern matching
                    // Auto-deref should allow direct field access on trans_ast as if it were &TransitionDefinitionAst
                    crate::StateBodyItemAst::Transition(trans_ast) => {
                        transitions_for_this_state.push(TmpTransition {
                            event_name: &trans_ast.event_name,
                            target_state_path_ast: &trans_ast.target_state_path,
                            target_state_idx: None,
                            guard_handler: trans_ast
                                .guard_clause
                                .as_ref()
                                .map(|gc| &gc.condition_function_expression),
                            action_handler: trans_ast
                                .action_clause
                                .as_ref()
                                .map(|ac| &ac.transition_action_expression),
                            on_keyword_span: trans_ast.on_keyword_token.span,
                        });
                    }
                    crate::StateBodyItemAst::NestedState(nested_state_decl_ast) => {
                        let child_idx = self.process_state_declaration(
                            nested_state_decl_ast,
                            Some(&full_path_name),
                            depth + 1,
                            &mut HashSet::new(),
                        )?;
                        children_indices_for_this_state.push(child_idx);
                    }
                }
            }

            if let Some(state_to_update) = self.all_states.get_mut(current_node_index) {
                state_to_update.children_indices = children_indices_for_this_state;
                state_to_update.entry_handler = entry_handler_opt;
                state_to_update.exit_handler = exit_handler_opt;
                state_to_update.transitions = transitions_for_this_state;
            } else {
                return Err(syn::Error::new(
                    state_decl_ast.name.span(),
                    "Internal error: Failed to find placeholder state for update",
                ));
            }
            Ok(current_node_index)
        }
    }
}

pub(crate) mod code_generator {
    use crate::intermediate_tree::TmpStateTreeBuilder;
    use proc_macro2::{Span, TokenStream};
    use quote::{format_ident, quote};
    use std::collections::HashMap;
    use syn::{Error as SynError, Ident, Path, Result as SynResult}; // Added Path
                                                                    // Removed: use crate::StateChartInputAst;
    use syn::spanned::Spanned;

    fn to_pascal_case(s: &str) -> Ident {
        let mut pascal = String::new();
        let mut capitalize_next = true;
        for c in s.chars() {
            if c == '_' {
                capitalize_next = true;
            } else if capitalize_next {
                pascal.push(c.to_ascii_uppercase());
                capitalize_next = false;
            } else {
                pascal.push(c);
            }
        }
        if pascal.is_empty() {
            format_ident!("UnnamedState")
        } else {
            format_ident!("{}", pascal)
        }
    }

    #[derive(Debug)]
    pub(crate) struct GeneratedStateIds {
        pub enum_definition_tokens: TokenStream,
        #[allow(dead_code)]
        pub state_id_enum_name: Ident,
        #[allow(dead_code)]
        pub full_path_to_variant_ident: HashMap<String, Ident>,
    }

    pub(crate) fn generate_state_id_logic(
        builder: &TmpStateTreeBuilder,
        machine_name: &Ident,
    ) -> GeneratedStateIds {
        let enum_name_str = format!("{machine_name}StateId");
        let state_id_enum_name = format_ident!("{}", enum_name_str);

        let mut full_path_to_variant_map = HashMap::new();

        let variants_code: Vec<Ident> = builder
            .all_states
            .iter()
            .map(|tmp_state| {
                let variant_ident = to_pascal_case(&tmp_state.full_path_name);
                full_path_to_variant_map
                    .insert(tmp_state.full_path_name.clone(), variant_ident.clone());
                variant_ident
            })
            .collect();

        let enum_definition_tokens = quote! {
            #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
            pub enum #state_id_enum_name {
                #(#variants_code),*
            }
        };

        GeneratedStateIds {
            enum_definition_tokens,
            state_id_enum_name,
            full_path_to_variant_ident: full_path_to_variant_map,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn generate_states_array<'ast>(
        builder: &'ast TmpStateTreeBuilder<'ast>,
        generated_ids: &GeneratedStateIds,
        context_type_path: &'ast syn::Path,
    ) -> SynResult<TokenStream> {
        let state_id_enum_name = &generated_ids.state_id_enum_name;
        let mut state_node_initializers = Vec::new();
        for tmp_state in &builder.all_states {
            // ... existing id, parent_id, initial_child_id, entry_action, exit_action expressions ...
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
                |p_expr| quote! { Some(#p_expr as ActionFn<#context_type_path>) },
            );
            let exit_action_expr = tmp_state.exit_handler.map_or_else(
                || quote! { None },
                |p_expr| quote! { Some(#p_expr as ActionFn<#context_type_path>) },
            );

            let is_parallel_literal = tmp_state.is_parallel; // This is already a bool

            state_node_initializers.push(quote! {
                StateNode {
                    id: #state_id_enum_name::#current_state_id_variant,
                    parent: #parent_id_expr,
                    initial_child: #initial_child_id_expr,
                    entry_action: #entry_action_expr,
                    exit_action: #exit_action_expr,
                    is_parallel: #is_parallel_literal, // Added field
                }
            });
        }
        let states_array_ts = quote! {
            const STATES: &[StateNode<#state_id_enum_name, #context_type_path>] = &[
                #(#state_node_initializers),*
            ];
        };
        Ok(states_array_ts)
    }

    #[allow(dead_code)]
    pub(crate) fn generate_transitions_array<'ast>(
        builder: &'ast TmpStateTreeBuilder<'ast>,
        generated_ids: &GeneratedStateIds,
        event_type_path: &'ast syn::Path,   // Changed
        context_type_path: &'ast syn::Path, // Changed
    ) -> SynResult<TokenStream> {
        let state_id_enum_name = &generated_ids.state_id_enum_name;
        let mut transition_initializers = Vec::new();
        for tmp_state in &builder.all_states {
            let from_state_id_variant = generated_ids.full_path_to_variant_ident.get(&tmp_state.full_path_name)
                .ok_or_else(|| SynError::new(tmp_state.name_span, "Internal error: 'from_state' full_path_name not found in generated IDs map"))?;
            for tmp_trans in &tmp_state.transitions {
                let target_state_idx = tmp_trans.target_state_idx.ok_or_else(|| {
                    SynError::new(
                        tmp_trans.on_keyword_span,
                        "Internal error: Transition target index not resolved.",
                    )
                })?;
                let target_tmp_state =
                    builder.all_states.get(target_state_idx).ok_or_else(|| {
                        SynError::new(
                            tmp_trans.on_keyword_span,
                            "Internal error: Invalid target_state_idx.",
                        )
                    })?;
                let to_state_id_variant = generated_ids.full_path_to_variant_ident.get(&target_tmp_state.full_path_name)
                    .ok_or_else(|| SynError::new(tmp_trans.on_keyword_span, "Internal error: 'to_state' full_path_name not found in map for resolved index."))?;
                let event_name_ident = tmp_trans.event_name;
                let event_expr = quote! { #event_type_path::#event_name_ident };
                let action_expr = tmp_trans.action_handler.map_or_else(
                    || quote! { None },
                    |p_expr| quote! { Some(#p_expr as ActionFn<#context_type_path>) },
                ); // p_expr is &Expr
                let guard_expr = tmp_trans.guard_handler.map_or_else(|| quote!{ None },
                    |p_expr| quote!{ Some(#p_expr as GuardFn<#context_type_path, #event_type_path>) }); // p_expr is &Expr
                transition_initializers.push(quote! {
                    Transition {
                        from_state: #state_id_enum_name::#from_state_id_variant,
                        event: #event_expr,
                        to_state: #state_id_enum_name::#to_state_id_variant,
                        action: #action_expr,
                        guard: #guard_expr,
                    }
                });
            }
        }
        let transitions_array_ts = quote! {
            const TRANSITIONS: &[Transition<#state_id_enum_name, #event_type_path, #context_type_path>] = &[
                #(#transition_initializers),*
            ];
        };
        Ok(transitions_array_ts)
    }

    // Helper to convert an Expr that should represent a state path into a lookup string.
    // For V1, initial target must be a simple Ident or a Path like foo::bar.
    // It cannot be something like MyMachine.State1 if MyMachine is the machine name.
    // It refers to a top-level state name, or a qualified path to one.
    fn expr_to_state_path_string(path_expr: &Path, _base_span: Span) -> SynResult<String> {
        // Changed expr: &Expr to path_expr: &Path
        // Input is already a Path, directly use path_to_string_for_lookup from intermediate_tree
        if path_expr.leading_colon.is_some() {
            return Err(SynError::new(
                path_expr.span(),
                "Absolute paths (`::foo`) are not supported for initial state targets.",
            ));
        }
        // qself check removed as it's not directly on syn::Path and less relevant for this simplified path usage.
        // The main goal is to join segments.
        Ok(path_expr
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect::<Vec<String>>()
            .join("_"))
    }

    #[allow(dead_code)]
    pub(crate) fn determine_initial_leaf_state_id<'ast>(
        builder: &'ast TmpStateTreeBuilder<'ast>,
        generated_ids: &GeneratedStateIds,
        input_ast: &'ast crate::StateChartInputAst, // StateChartInputAst now has initial_target_expression as Path
    ) -> SynResult<TokenStream> {
        let initial_target_path = &input_ast.initial_target_expression; // This is &Path
        let initial_target_span = initial_target_path.span();

        // Pass initial_target_path directly to expr_to_state_path_string
        let top_level_target_name_str =
            expr_to_state_path_string(initial_target_path, initial_target_span)?;

        let mut current_state_idx = builder
            .state_full_path_to_idx_map
            .get(&top_level_target_name_str)
            .copied()
            .ok_or_else(|| {
                SynError::new(
                    initial_target_span,
                    format!(
                        "Declared top-level initial state '{top_level_target_name_str}' not found."
                    ),
                )
            })?;

        if builder.all_states[current_state_idx]
            .parent_full_path_name
            .is_some()
        {
            return Err(SynError::new(initial_target_span, format!("Declared top-level initial state '{top_level_target_name_str}' is not a top-level state.")));
        }

        while let Some(child_idx) = builder.all_states[current_state_idx].initial_child_idx {
            current_state_idx = child_idx;
        }

        let leaf_state_full_path_name = &builder.all_states[current_state_idx].full_path_name;
        let leaf_state_variant_ident = generated_ids
            .full_path_to_variant_ident
            .get(leaf_state_full_path_name)
            .ok_or_else(|| {
                SynError::new(
                    initial_target_span,
                    "Internal error: Resolved initial leaf state not found in ID map.",
                )
            })?;

        let state_id_enum_name = &generated_ids.state_id_enum_name;
        Ok(quote! { #state_id_enum_name::#leaf_state_variant_ident })
    }

    #[allow(dead_code)] // TODO: Test this function
    pub(crate) fn generate_machine_definition_const(
        machine_name: &Ident,
        generated_ids: &GeneratedStateIds,
        event_type_path: &syn::Path,   // Changed
        context_type_path: &syn::Path, // Changed
        initial_leaf_state_id_ts: &TokenStream,
    ) -> TokenStream {
        let state_id_enum_name = &generated_ids.state_id_enum_name;
        let machine_def_const_name_str = format!(
            "{}_MACHINE_DEFINITION",
            machine_name.to_string().to_uppercase()
        );
        let machine_def_const_ident = format_ident!("{}", machine_def_const_name_str);
        let machine_def_ts = quote! {
            pub const #machine_def_const_ident: lit_bit_core::core::MachineDefinition<
                #state_id_enum_name,
                #event_type_path,
                #context_type_path
            > = lit_bit_core::core::MachineDefinition::new(
                STATES,
                TRANSITIONS,
                #initial_leaf_state_id_ts
            );
        };
        machine_def_ts
    }

    #[allow(dead_code)]
    pub(crate) fn generate_machine_struct_and_impl(
        machine_name: &Ident,
        state_id_enum_name: &Ident,
        event_type_path: &syn::Path,
        context_type_path: &syn::Path,
        machine_definition_const_ident: &Ident,
    ) -> TokenStream {
        let max_active_regions_literal = proc_macro2::Literal::usize_unsuffixed(4);

        let machine_struct_ts = quote! {
            #[derive(Debug)]
            pub struct #machine_name {
                runtime: lit_bit_core::core::Runtime<#state_id_enum_name, #event_type_path, #context_type_path>,
            }
            impl #machine_name {
                pub fn new(context: #context_type_path) -> Self {
                    Self {
                        runtime: lit_bit_core::core::Runtime::new(#machine_definition_const_ident.clone(), context),
                    }
                }
            }
            // Ensure this impl block correctly implements the StateMachine trait from lit_bit_core (not lit_bit_core::core)
            impl lit_bit_core::StateMachine for #machine_name {
                type State = #state_id_enum_name;
                type Event = #event_type_path;
                type Context = #context_type_path;
                fn send(&mut self, event: Self::Event) -> bool {
                    self.runtime.send(event)
                }
                // Corrected signature for state() method
                fn state(&self) -> heapless::Vec<Self::State, #max_active_regions_literal> {
                    self.runtime.state() // This assumes runtime.state() also returns the Vec
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

    // TODO: fn generate_machine_struct_and_impl(...)
}

// In the main proc_macro function, after parsing:
#[proc_macro]
pub fn statechart(input: TokenStream) -> TokenStream {
    let parsed_ast = match syn::parse::<crate::StateChartInputAst>(input) {
        Ok(ast) => ast,
        Err(err) => return err.to_compile_error().into(),
    };
    let mut builder = crate::intermediate_tree::TmpStateTreeBuilder::new();
    if let Err(err) = builder.build_from_ast(&parsed_ast) {
        return err.to_compile_error().into();
    }
    let machine_name_ident = &parsed_ast.name;
    // These are now Paths from the AST
    let context_type_path = &parsed_ast.context_type;
    let event_type_path = &parsed_ast.event_type;

    let generated_ids_info =
        crate::code_generator::generate_state_id_logic(&builder, machine_name_ident);

    // Pass Paths to generator functions
    let states_array_ts = match crate::code_generator::generate_states_array(
        &builder,
        &generated_ids_info,
        context_type_path,
    ) {
        Ok(ts) => ts,
        Err(err) => return err.to_compile_error().into(),
    };
    let transitions_array_ts = match crate::code_generator::generate_transitions_array(
        &builder,
        &generated_ids_info,
        event_type_path,
        context_type_path,
    ) {
        Ok(ts) => ts,
        Err(err) => return err.to_compile_error().into(),
    };
    let initial_leaf_state_id_ts = match crate::code_generator::determine_initial_leaf_state_id(
        &builder,
        &generated_ids_info,
        &parsed_ast,
    ) {
        Ok(ts) => ts,
        Err(err) => return err.to_compile_error().into(),
    };

    let machine_definition_const_ident_str = format!(
        "{}_MACHINE_DEFINITION",
        machine_name_ident.to_string().to_uppercase()
    );
    let machine_definition_const_ident =
        quote::format_ident!("{}", machine_definition_const_ident_str);

    let machine_def_const_ts = crate::code_generator::generate_machine_definition_const(
        machine_name_ident,
        &generated_ids_info,
        event_type_path,   // Pass Path
        context_type_path, // Pass Path
        &initial_leaf_state_id_ts,
    );

    let machine_struct_impl_ts = crate::code_generator::generate_machine_struct_and_impl(
        machine_name_ident,
        &generated_ids_info.state_id_enum_name,
        event_type_path,   // Pass Path
        context_type_path, // Pass Path
        &machine_definition_const_ident,
    );

    let state_id_enum_ts = generated_ids_info.enum_definition_tokens;

    let core_types_definitions = quote! {
        use lit_bit_core::core::{StateMachine, Runtime, StateNode, Transition, ActionFn, GuardFn};
    };

    let final_code = quote! {
        mod generated_state_machine {
            #core_types_definitions
            #[allow(unused_imports)]
            use super::*;
            // It's important that user-defined types/paths like TestContext, TestEvent, entry_s1
            // are resolvable from within this generated module.
            // If they are defined at the crate root of where this macro is called (e.g. integration test crate root)
            // then paths like `crate::TestContext` or simply `TestContext` (if a `use super::*` is effective
            // at the module where `statechart!` is invoked) should work when these #context_type_path tokens expand.
            // The `use super::*` in the test module `mod basic_machine_integration_test` should make them visible.
            #state_id_enum_ts
            #states_array_ts
            #transitions_array_ts
            #machine_def_const_ts
            #machine_struct_impl_ts
        }
        pub use generated_state_machine::*;
    };
    final_code.into()
}

// Need to make ast_structs module visible to intermediate_tree, or pass items differently.
// For now, let's assume ast_structs is a module containing the previously defined AST structs.
// Or, just use `crate::StructName` if they are at the crate root of lit-bit-macro/src/lib.rs
// For this edit, I will assume they are at the crate root for simplicity of the diff.
// So, `crate::StateChartInputAst`, `crate::StateDeclarationAst` etc. will be used in intermediate_tree.

// Let's adjust paths for AST structs assuming they are in the root of lit-bit-macro/src/lib.rs:
// The edit will make these changes within the `intermediate_tree` module and `statechart` function.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code_generator::generate_state_id_logic;
    use crate::intermediate_tree::TmpStateTreeBuilder;
    use syn::parse_str; // Import the new function

    #[allow(dead_code)]
    fn ident(s: &str) -> Ident {
        Ident::new(s, proc_macro2::Span::call_site())
    }
    #[allow(dead_code)]
    fn simple_path(s: &str) -> Path {
        Path::from(ident(s))
    }

    // Helper to quickly parse a full statechart DSL string into AST for builder tests
    fn parse_dsl(input_dsl: &str) -> Result<StateChartInputAst> {
        parse_str::<StateChartInputAst>(input_dsl)
    }

    #[test]
    fn parse_state_chart_input_header_only() {
        let input_str = "name: MyMachine, context: Ctx, event: Ev, initial: StartState,";
        let result = parse_str::<StateChartInputAst>(input_str);
        assert!(result.is_ok(), "Failed to parse header: {:?}", result.err());
        let ast = result.unwrap();
        assert_eq!(ast.name.to_string(), "MyMachine");
        let ct = &ast.context_type;
        assert_eq!(quote!(#ct).to_string(), "Ctx");
        let et = &ast.event_type;
        assert_eq!(quote!(#et).to_string(), "Ev");
        let ite = &ast.initial_target_expression;
        assert_eq!(quote!(#ite).to_string(), "StartState");
        assert!(
            ast.comma4.is_some(),
            "Expected trailing comma (comma4) after initial state path"
        );
        assert!(
            ast.top_level_states.is_empty(),
            "Expected no states for header-only input"
        );
    }

    #[test]
    fn parse_state_chart_input_header_no_trailing_comma() {
        let input_str = "name: MyMachine, context: Ctx, event: Ev, initial: StartState";
        let result = parse_str::<StateChartInputAst>(input_str);
        assert!(
            result.is_ok(),
            "Failed to parse header without trailing comma: {:?}",
            result.err()
        );
        let ast = result.unwrap();
        assert_eq!(ast.name.to_string(), "MyMachine");
        let et = &ast.event_type;
        assert_eq!(quote!(#et).to_string(), "Ev");
        assert!(
            ast.comma4.is_none(),
            "Expected no trailing comma (comma4) after initial state path"
        );
        assert!(ast.top_level_states.is_empty());
    }

    #[test]
    fn parse_state_chart_input_with_one_state() {
        let input_str = "name: Test, context: Ctx, event: Ev, initial: S1, state S1 {}";
        let result = parse_str::<StateChartInputAst>(input_str);
        assert!(
            result.is_ok(),
            "Parse failed with one state: {:?}",
            result.err()
        );
        let ast = result.unwrap();
        let et = &ast.event_type;
        assert_eq!(quote!(#et).to_string(), "Ev");
        assert_eq!(ast.top_level_states.len(), 1);
        assert_eq!(ast.top_level_states[0].name.to_string(), "S1");
    }

    #[test]
    fn parse_state_chart_input_with_multiple_states() {
        let input_str = "name: Test, context: Ctx, event: Ev, initial: S1, state S1 {} state S2 {}";
        let result = parse_str::<StateChartInputAst>(input_str);
        assert!(result.is_ok(), "Parse failed: {:?}", result.err());
        let ast = result.unwrap();
        assert_eq!(ast.top_level_states.len(), 2);
        assert_eq!(ast.top_level_states[0].name.to_string(), "S1");
        assert_eq!(ast.top_level_states[1].name.to_string(), "S2");
    }

    #[test]
    fn parse_state_chart_input_missing_comma_in_header() {
        let input_str = "name: MyMachine context: Ctx, event: Ev, initial: Start,";
        let result = parse_str::<StateChartInputAst>(input_str);
        assert!(result.is_err(), "Expected error for missing comma");
    }

    #[test]
    fn parse_state_chart_input_extra_tokens_after_states() {
        let input_str =
            "name: Test, context: Ctx, event: Ev, initial: S1, state S1 {} unexpected_token";
        let result = parse_str::<StateChartInputAst>(input_str);
        assert!(
            result.is_err(),
            "Expected error for extra tokens, but got Ok({:?})",
            result.as_ref().ok()
        );
    }

    #[test]
    fn parse_empty_state_declaration() {
        let input_str = "state EmptyState {}";
        let result = parse_str::<StateDeclarationAst>(input_str);
        assert!(result.is_ok(), "Parse failed: {:?}", result.err());
        let ast = result.unwrap();
        assert_eq!(ast.name.to_string(), "EmptyState");
        assert!(ast.default_child_declaration.is_none());
        assert!(ast.body_items.is_empty());
    }

    #[test]
    fn parse_state_with_initial_declaration() {
        let input_str = "state Parent { initial: ChildA; }";
        let result = parse_str::<StateDeclarationAst>(input_str);
        assert!(
            result.is_ok(),
            "Parse failed for initial declaration: {:?}",
            result.err()
        );
        let ast = result.unwrap();
        assert_eq!(ast.name.to_string(), "Parent");
        assert!(ast.default_child_declaration.is_some());
        let initial_decl = ast.default_child_declaration.as_ref().unwrap();
        let child_expr_val = &initial_decl.child_state_expression;
        assert_eq!(quote!(#child_expr_val).to_string(), "ChildA");
        assert!(ast.body_items.is_empty());
    }

    #[test]
    fn parse_state_with_entry_hook() {
        let input_str = "state LoggingState { entry: self.log_entry; }";
        let result = parse_str::<StateDeclarationAst>(input_str);
        assert!(
            result.is_ok(),
            "Parse failed for entry hook: {:?}",
            result.err()
        );
        let ast = result.unwrap();
        assert_eq!(ast.body_items.len(), 1);
        match &ast.body_items[0] {
            StateBodyItemAst::EntryHook(hook_ast) => {
                assert_eq!(hook_ast.kind.to_string(), "entry");
                let hook_expr_val = &hook_ast.hook_function_expression;
                assert_eq!(quote!(#hook_expr_val).to_string(), "self . log_entry");
            }
            _ => panic!("Expected EntryHook"),
        }
    }

    #[test]
    fn parse_state_with_leading_dot_entry_hook_error() {
        let input_str = "state LoggingState { entry: .log_entry; }";
        let result = parse_str::<StateDeclarationAst>(input_str);
        assert!(
            result.is_err(),
            "Expected parse to fail for leading dot hook, but got Ok({:?})",
            result.ok()
        );
    }

    #[test]
    fn parse_state_with_exit_hook() {
        let input_str = "state CleanUpState { exit: self.cleanup_method; }";
        let result = parse_str::<StateDeclarationAst>(input_str);
        assert!(
            result.is_ok(),
            "Parse failed for exit hook: {:?}",
            result.err()
        );
        let ast = result.unwrap();
        assert_eq!(ast.body_items.len(), 1);
        match &ast.body_items[0] {
            StateBodyItemAst::ExitHook(hook_ast) => {
                assert_eq!(hook_ast.kind.to_string(), "exit");
                let hook_expr_val = &hook_ast.hook_function_expression;
                assert_eq!(quote!(#hook_expr_val).to_string(), "self . cleanup_method");
            }
            _ => panic!("Expected ExitHook"),
        }
    }

    #[test]
    fn parse_state_with_nested_state() {
        let input_str = "state Outer { state Inner {} }";
        let result = parse_str::<StateDeclarationAst>(input_str);
        assert!(result.is_ok(), "Parse failed: {:?}", result.err());
        let ast = result.unwrap();
        assert_eq!(ast.body_items.len(), 1);
        match &ast.body_items[0] {
            StateBodyItemAst::NestedState(nested_state_ast) => {
                assert_eq!(nested_state_ast.name.to_string(), "Inner");
            }
            _ => panic!("Expected NestedState"),
        }
    }

    #[test]
    fn parse_state_with_multiple_body_items() {
        let input_str = "state Complex {
            initial: C1;
            entry: self.on_enter;
            state C1 {}
            exit: self.on_exit;
        }";
        let result = parse_str::<StateDeclarationAst>(input_str);
        assert!(
            result.is_ok(),
            "Parse failed for multiple body items: {:?}",
            result.err()
        );
        let ast = result.unwrap();
        assert!(ast.default_child_declaration.is_some());
        assert_eq!(ast.body_items.len(), 3);
        match &ast.body_items[0] {
            StateBodyItemAst::EntryHook(hook_ast) => {
                assert_eq!(hook_ast.kind.to_string(), "entry");
                let hook_expr_val = &hook_ast.hook_function_expression;
                assert_eq!(quote!(#hook_expr_val).to_string(), "self . on_enter");
            }
            _ => panic!("Expected first item to be EntryHook"),
        }
        match &ast.body_items[1] {
            StateBodyItemAst::NestedState(nested_state_ast) => {
                assert_eq!(nested_state_ast.name.to_string(), "C1");
            }
            _ => panic!("Expected second item to be NestedState"),
        }
        match &ast.body_items[2] {
            StateBodyItemAst::ExitHook(hook_ast) => {
                assert_eq!(hook_ast.kind.to_string(), "exit");
                let hook_expr_val = &hook_ast.hook_function_expression;
                assert_eq!(quote!(#hook_expr_val).to_string(), "self . on_exit");
            }
            _ => panic!("Expected third item to be ExitHook"),
        }
    }

    #[test]
    fn parse_state_with_invalid_token_in_body() {
        let input_str = "state Bad { initial: C1; not_a_keyword: foo; }";
        let result = parse_str::<StateDeclarationAst>(input_str);
        assert!(
            result.is_err(),
            "Expected error for invalid token in state body"
        );
    }

    #[test]
    fn parse_default_child_declaration() {
        let input_str = "initial: MyChild::State;"; // Changed . to ::
        let result = parse_str::<DefaultChildDeclarationAst>(input_str);
        assert!(
            result.is_ok(),
            "Parse failed for default child decl: {:?}",
            result.err()
        );
        let ast = result.unwrap();
        let child_expr_val = &ast.child_state_expression;
        assert_eq!(quote!(#child_expr_val).to_string(), "MyChild :: State");
    }

    #[test]
    fn parse_default_child_declaration_simple_ident() {
        let input_str = "initial: ChildA;";
        let result = parse_str::<DefaultChildDeclarationAst>(input_str);
        assert!(
            result.is_ok(),
            "Parse failed for simple ident default child decl: {:?}",
            result.err()
        );
        let ast = result.unwrap();
        let child_expr_val = &ast.child_state_expression;
        assert_eq!(quote!(#child_expr_val).to_string(), "ChildA");
    }

    #[test]
    fn parse_lifecycle_hook_entry() {
        let input_str = "entry: module::on_entry_hook;";
        let result = parse_str::<LifecycleHookAst>(input_str);
        assert!(
            result.is_ok(),
            "Parse failed for lifecycle hook: {:?}",
            result.err()
        );
        let ast = result.unwrap();
        assert_eq!(ast.kind.to_string(), "entry");
        let hook_expr_val = &ast.hook_function_expression;
        assert_eq!(
            quote!(#hook_expr_val).to_string(),
            "module :: on_entry_hook"
        );
    }

    #[test]
    fn parse_lifecycle_hook_invalid_kind() {
        let input_str = "起動: .my_handler;";
        let result = parse_str::<LifecycleHookAst>(input_str);
        assert!(
            result.is_err(),
            "Expected error for invalid lifecycle hook kind"
        );
    }

    // --- Tests for TransitionDefinitionAst ---
    #[test]
    fn parse_transition_simple() {
        let input_str = "on MyEvent => TargetState;";
        let result = parse_str::<TransitionDefinitionAst>(input_str);
        assert!(
            result.is_ok(),
            "Parse failed for simple transition: {:?}",
            result.err()
        );
        let ast = result.unwrap();
        assert_eq!(ast.event_name.to_string(), "MyEvent");
        let target_path_val = &ast.target_state_path;
        assert_eq!(quote!(#target_path_val).to_string(), "TargetState");
        assert!(ast.guard_clause.is_none(), "Expected no guard clause");
        assert!(ast.action_clause.is_none(), "Expected no action clause");
    }

    #[test]
    fn parse_transition_with_guard_only() {
        let input_str = "on EvName [guard self.can_transition] => NextState;";
        let result = parse_str::<TransitionDefinitionAst>(input_str);
        assert!(
            result.is_ok(),
            "Parse failed for guard-only transition: {:?}",
            result.err()
        );
        let ast = result.unwrap();
        assert_eq!(ast.event_name.to_string(), "EvName");
        assert!(ast.guard_clause.is_some(), "Expected a guard clause");
        let guard_clause = ast.guard_clause.as_ref().unwrap();
        let cond_expr_val = &guard_clause.condition_function_expression;
        assert_eq!(quote!(#cond_expr_val).to_string(), "self . can_transition");
        assert!(ast.action_clause.is_none(), "Expected no action clause");
        let target_path_val = &ast.target_state_path;
        assert_eq!(quote!(#target_path_val).to_string(), "NextState");
    }

    #[test]
    fn parse_transition_with_action_only_explicit_keyword() {
        let input_str = "on Click => Target [action self.do_action];";
        let result = parse_str::<TransitionDefinitionAst>(input_str);
        assert!(
            result.is_ok(),
            "Parse failed for action-only (explicit) transition: {:?}",
            result.err()
        );
        let ast = result.unwrap();
        assert_eq!(ast.event_name.to_string(), "Click");
        assert!(ast.guard_clause.is_none(), "Expected no guard clause");
        assert!(ast.action_clause.is_some(), "Expected an action clause");
        let action_clause = ast.action_clause.as_ref().unwrap();
        assert!(
            action_clause.action_keyword_token.is_some(),
            "Expected 'action' keyword token"
        );
        let action_expr_val = &action_clause.transition_action_expression;
        assert_eq!(quote!(#action_expr_val).to_string(), "self . do_action");
        let target_path_val = &ast.target_state_path;
        assert_eq!(quote!(#target_path_val).to_string(), "Target");
    }

    #[test]
    fn parse_transition_with_action_only_implicit_keyword() {
        let input_str = "on Submit => ResultPage [.handle_submission];";
        let result = parse_str::<TransitionDefinitionAst>(input_str);
        assert!(
            result.is_err(),
            "Expected parse to fail for leading dot action, but got Ok({:?})",
            result.ok()
        );
        if let Err(e) = result {
            assert!(e
                .to_string()
                .contains("Leading dot notation for action handlers"));
        }
    }

    #[test]
    fn parse_transition_with_guard_and_action() {
        let input_str = "on DataReceived [guard is_valid] => ProcessData [action log_event];";
        let result = parse_str::<TransitionDefinitionAst>(input_str);
        assert!(
            result.is_ok(),
            "Parse failed for guard+action transition: {:?}",
            result.err()
        );
        let ast = result.unwrap();
        assert_eq!(ast.event_name.to_string(), "DataReceived");

        assert!(ast.guard_clause.is_some(), "Expected guard clause");
        let guard_clause = ast.guard_clause.as_ref().unwrap();
        let guard_expr_val = &guard_clause.condition_function_expression;
        assert_eq!(quote!(#guard_expr_val).to_string(), "is_valid");
        assert!(ast.action_clause.is_some(), "Expected action clause");
        let action_clause = ast.action_clause.as_ref().unwrap();
        assert!(
            action_clause.action_keyword_token.is_some(),
            "Expected explicit 'action' keyword token"
        );
        let action_expr_val = &action_clause.transition_action_expression;
        assert_eq!(quote!(#action_expr_val).to_string(), "log_event");
    }

    #[test]
    fn parse_transition_with_guard_and_explicit_action() {
        let input_str = "on Update [guard needs_update] => SaveState [action self.perform_save];";
        let result = parse_str::<TransitionDefinitionAst>(input_str);
        assert!(
            result.is_ok(),
            "Parse failed for guard+explicit_action: {:?}",
            result.err()
        );
        let ast = result.unwrap();

        assert!(ast.guard_clause.is_some());
        let guard_clause = ast.guard_clause.as_ref().unwrap();
        let guard_expr_val = &guard_clause.condition_function_expression;
        assert_eq!(quote!(#guard_expr_val).to_string(), "needs_update");
        assert!(ast.action_clause.is_some());
        let action_clause = ast.action_clause.as_ref().unwrap();
        assert!(action_clause.action_keyword_token.is_some());
        let action_expr_val = &action_clause.transition_action_expression;
        assert_eq!(quote!(#action_expr_val).to_string(), "self . perform_save");
    }

    // --- TODO: Tests for GuardConditionAst (direct parsing, though usually parsed via Transition) ---
    #[test]
    fn parse_guard_condition_ast() {
        let input_str = "[guard my_app::guards::is_user_active]";
        let result = parse_str::<GuardConditionAst>(input_str);
        assert!(
            result.is_ok(),
            "Parse failed for GuardConditionAst: {:?}",
            result.err()
        );
        let ast = result.unwrap();
        let cond_expr_val = &ast.condition_function_expression;
        assert_eq!(
            quote!(#cond_expr_val).to_string(),
            "my_app :: guards :: is_user_active"
        );
    }

    // --- TODO: Tests for TransitionActionAst (direct parsing) ---
    #[test]
    fn parse_transition_action_ast_explicit_keyword() {
        let input_str = "[action my_app::actions::log_transition]";
        let result = parse_str::<TransitionActionAst>(input_str);
        assert!(
            result.is_ok(),
            "Parse failed for TransitionActionAst (explicit): {:?}",
            result.err()
        );
        let ast = result.unwrap();
        assert!(ast.action_keyword_token.is_some());
        let action_expr_val = &ast.transition_action_expression;
        assert_eq!(
            quote!(#action_expr_val).to_string(),
            "my_app :: actions :: log_transition"
        );
    }

    #[test]
    fn parse_transition_action_ast_implicit_keyword() {
        let input_str = "[self.increment_counter]";
        let result = parse_str::<TransitionActionAst>(input_str);
        assert!(
            result.is_ok(),
            "Parse failed for TransitionActionAst (implicit): {:?}",
            result.err()
        );
        let ast = result.unwrap();
        assert!(ast.action_keyword_token.is_none());
        let action_expr_val = &ast.transition_action_expression;
        assert_eq!(
            quote!(#action_expr_val).to_string(),
            "self . increment_counter"
        );
    }

    #[test]
    fn parse_transition_action_ast_leading_dot_error() {
        let input_str = "[.should_error]";
        let result = parse_str::<TransitionActionAst>(input_str);
        assert!(
            result.is_err(),
            "Expected error for leading dot in TransitionActionAst"
        );
        if let Err(e) = result {
            assert!(
                e.to_string().contains("expected an expression")
                    || e.to_string()
                        .contains("Unexpected tokens after transition action expression")
            );
        }
    }

    #[test]
    fn parse_transition_missing_semicolon() {
        let input_str = "on MyEvent => TargetState"; // Missing semicolon
        let result = parse_str::<TransitionDefinitionAst>(input_str);
        assert!(
            result.is_err(),
            "Expected error for missing semicolon in transition"
        );
    }

    #[test]
    fn parse_transition_malformed_guard() {
        let input_str = "on MyEvent [guard] => TargetState;"; // Missing path in guard
        let result = parse_str::<TransitionDefinitionAst>(input_str);
        assert!(result.is_err(), "Expected error for malformed guard");
    }

    #[test]
    fn parse_transition_malformed_action() {
        let input_str = "on MyEvent => TargetState [action];"; // Missing path in action
        let result = parse_str::<TransitionDefinitionAst>(input_str);
        assert!(result.is_err(), "Expected error for malformed action");
    }

    // --- Tests for TmpStateTreeBuilder - Semantic Analysis ---

    // --- Tests for Initial Child Resolution ---
    #[test]
    fn initial_child_valid_direct_child() {
        let dsl = r"
            name: TestMachine,
            context: Ctx,
            event: Ev,
            initial: S1, 
            state S1 {
                initial: S1_A;
                state S1_A {}
                state S1_B {}
            }
        ";
        let ast = parse_dsl(dsl).expect("DSL parsing failed");
        let mut builder = TmpStateTreeBuilder::new();
        let build_result = builder.build_from_ast(&ast);
        assert!(
            build_result.is_ok(),
            "Builder failed: {:?}",
            build_result.err()
        );

        assert_eq!(builder.all_states.len(), 3); // S1, S1_A, S1_B
        let s1_idx = builder.state_full_path_to_idx_map.get("S1").unwrap();
        let s1_a_idx = builder.state_full_path_to_idx_map.get("S1_S1_A").unwrap();

        let s1_node = &builder.all_states[*s1_idx];
        assert_eq!(
            s1_node.initial_child_idx,
            Some(*s1_a_idx),
            "S1 initial child should be S1_A"
        );
        assert!(s1_node.declared_initial_child_expression.is_some());
    }

    #[test]
    fn initial_child_missing_for_composite_state() {
        let dsl = r"
            name: TestMachine,
            context: Ctx,
            event: Ev,
            initial: S1,
            state S1 {
                state S1_A {}
            }
        ";
        let ast = parse_dsl(dsl).expect("DSL parsing failed");
        let mut builder = TmpStateTreeBuilder::new();
        let build_result = builder.build_from_ast(&ast);
        assert!(
            build_result.is_err(),
            "Expected error for missing initial declaration"
        );
        if let Err(e) = build_result {
            // Exact match for the format string part, variable part will differ
            let expected_message = format!(
                "Compound state '{}' must declare an 'initial' child state.",
                "S1"
            );
            assert_eq!(e.to_string(), expected_message, "Error message mismatch");
        }
    }

    #[test]
    fn initial_child_declared_for_leaf_state() {
        let dsl = r"
            name: TestMachine,
            context: Ctx,
            event: Ev,
            initial: S1,
            state S1 {
                initial: S1_A; 
            }
        ";
        let ast = parse_dsl(dsl).expect("DSL parsing failed");
        let mut builder = TmpStateTreeBuilder::new();
        let build_result = builder.build_from_ast(&ast);
        assert!(
            build_result.is_err(),
            "Expected error for initial on leaf state"
        );
        if let Err(e) = build_result {
            assert!(e.to_string().contains(
                "State 'S1' declares an 'initial' child but has no nested states defined."
            ));
        }
    }

    #[test]
    fn initial_child_target_not_a_direct_child() {
        let dsl = r"
            name: TestMachine,
            context: Ctx,
            event: Ev,
            initial: S1,
            state S1 {
                initial: S2_A; 
                state S1_A {}
            }
            state S2 {
                state S2_A {}
            }
        ";
        let ast = parse_dsl(dsl).expect("DSL parsing failed");
        let mut builder = TmpStateTreeBuilder::new();
        let build_result = builder.build_from_ast(&ast);
        assert!(
            build_result.is_err(),
            "Expected error for initial target not being a direct child"
        );
        if let Err(e) = build_result {
            let error_string = e.to_string();
            let expected_message = format!("Initial child '{}' declared for state '{}' is not defined as a direct child of this state.", "S2_A", "S1");
            // Trim both strings to remove potential leading/trailing whitespace differences
            assert_eq!(error_string.trim(), expected_message.trim(), "Error message mismatch. Actual trimmed: [{actual}], Expected trimmed: [{expected}]", actual = error_string.trim(), expected = expected_message.trim());
        }
    }

    #[test]
    fn initial_child_target_is_not_simple_identifier() {
        let dsl = r"
            name: TestMachine,
            context: Ctx,
            event: Ev,
            initial: S1,
            state S1 {
                initial: self.S1_A; // Problematic line: self.S1_A is not a valid Path for an initial child
                state S1_A {}
            }
        ";
        let result = parse_dsl(dsl); // Don't .expect() immediately
        assert!(result.is_err(), "Expected DSL parsing to fail for 'initial: self.S1_A;' because 'self.S1_A' is not a valid Path.");
        if let Err(e) = result {
            // Print the exact error string for debugging
            println!("Actual error string from parser: \"{e}\"");
            // The direct error from DefaultChildDeclarationAst trying to parse `self.S1_A` as Path and then expecting `;`
            assert!(e.to_string().contains("expected `;`") || e.to_string().contains("expected an identifier"),
                    "Error message did not indicate a Path parsing issue followed by missing semicolon. Got: {e}");
        }
    }

    // --- Tests for Code Generation (Stage 3) ---

    // Tests for StateId Enum Generation (re-adding with updated DSL)
    #[test]
    fn generate_simple_state_id_enum_updated() {
        let dsl = concat!(
            "name: TestSimple, ",
            "context: Ctx, ",
            "event: Ev, ",
            "initial: S1, ",
            "state S1 {}",
            "state S2 {}"
        );
        let ast = parse_dsl(dsl).expect("DSL parsing failed");
        let mut builder = TmpStateTreeBuilder::new();
        builder.build_from_ast(&ast).expect("Builder failed");
        let machine_name_ident = &ast.name;
        let ids_info = crate::code_generator::generate_state_id_logic(&builder, machine_name_ident);

        let expected_enum_str = quote! {
            #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
            pub enum TestSimpleStateId {
                S1,
                S2
            }
        }
        .to_string();
        assert_eq!(
            ids_info.enum_definition_tokens.to_string(),
            expected_enum_str
        );
        assert_eq!(ids_info.state_id_enum_name.to_string(), "TestSimpleStateId");
        assert_eq!(
            ids_info
                .full_path_to_variant_ident
                .get("S1")
                .unwrap()
                .to_string(),
            "S1"
        );
        assert_eq!(
            ids_info
                .full_path_to_variant_ident
                .get("S2")
                .unwrap()
                .to_string(),
            "S2"
        );
    }

    #[test]
    fn generate_nested_state_id_enum_updated() {
        let dsl = concat!(
            "name: TestNested, \n",
            "context: Ctx, \n",
            "event: Ev,\n", // Assuming Ev is a valid Type for event_type
            "initial: P1, \n",
            "state P1 { \n",
            "    initial: C1; \n",
            "    state C1 { \n",
            "        initial: GC1; \n",
            "        state GC1 {} \n",
            "        state GC2 {} \n",
            "    } \n",
            "    state C2 {} \n",
            "} \n",
            "state P2 {}"
        );
        let ast = parse_dsl(dsl).expect("DSL parsing failed for nested state_id_enum test "); // Added space
        let mut builder = TmpStateTreeBuilder::new();
        builder
            .build_from_ast(&ast)
            .expect("Builder failed for nested state_id_enum test "); // Added space
        let machine_name_ident = &ast.name;
        let ids_info = crate::code_generator::generate_state_id_logic(&builder, machine_name_ident);
        let expected_enum_str = quote! {
            #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
            pub enum TestNestedStateId {
                P1,
                P1C1,
                P1C1GC1,
                P1C1GC2,
                P1C2,
                P2
            }
        }
        .to_string();
        assert_eq!(
            ids_info.enum_definition_tokens.to_string(),
            expected_enum_str
        );
        assert_eq!(ids_info.state_id_enum_name.to_string(), "TestNestedStateId");
        assert_eq!(
            ids_info
                .full_path_to_variant_ident
                .get("P1")
                .unwrap()
                .to_string(),
            "P1"
        );
        assert_eq!(
            ids_info
                .full_path_to_variant_ident
                .get("P1_C1")
                .unwrap()
                .to_string(),
            "P1C1"
        );
        assert_eq!(
            ids_info
                .full_path_to_variant_ident
                .get("P1_C2")
                .unwrap()
                .to_string(),
            "P1C2"
        );
        assert_eq!(
            ids_info
                .full_path_to_variant_ident
                .get("P1_C1_GC1")
                .unwrap()
                .to_string(),
            "P1C1GC1"
        );
        assert_eq!(
            ids_info
                .full_path_to_variant_ident
                .get("P1_C1_GC2")
                .unwrap()
                .to_string(),
            "P1C1GC2"
        );
        assert_eq!(
            ids_info
                .full_path_to_variant_ident
                .get("P2")
                .unwrap()
                .to_string(),
            "P2"
        );
    }

    // Tests for STATES Array Generation (re-adding with updated DSL)
    #[test]
    fn generate_states_array_simple_no_actions() {
        let dsl = concat!(
            "name: TestMachine, ",
            "context: Ctx, ",
            "event: Ev, ",
            "initial: S1, ",
            "state S1 {}",
            "state S2 {}"
        );
        let ast = parse_dsl(dsl).expect("DSL parsing failed ");
        let mut builder = TmpStateTreeBuilder::new();
        builder.build_from_ast(&ast).expect("Builder failed ");
        let machine_name_ident = &ast.name;
        let context_type_ast = &ast.context_type;
        let ids_info = crate::code_generator::generate_state_id_logic(&builder, machine_name_ident);
        let states_array_tokens =
            crate::code_generator::generate_states_array(&builder, &ids_info, context_type_ast)
                .expect("generate_states_array failed ");
        let expected_states_array_str = quote! {
            const STATES: &[StateNode<TestMachineStateId, Ctx>] = &[
                StateNode {
                    id: TestMachineStateId::S1,
                    parent: None,
                    initial_child: None,
                    entry_action: None,
                    exit_action: None,
                    is_parallel: false, // Added
                },
                StateNode {
                    id: TestMachineStateId::S2,
                    parent: None,
                    initial_child: None,
                    entry_action: None,
                    exit_action: None,
                    is_parallel: false, // Added
                }
            ];
        }
        .to_string();
        let normalize = |s: String| s.split_whitespace().collect::<Vec<&str>>().join(" ");
        assert_eq!(
            normalize(states_array_tokens.to_string()),
            normalize(expected_states_array_str)
        );
    }

    #[test]
    fn generate_states_array_with_hierarchy_and_initial() {
        let dsl = concat!(
            "name: HierarchicalMachine, ",
            "context: AppContext, ",
            "event: AppEvent, ",
            "initial: Parent, ",
            "state Parent {",
            "    initial: ChildA;",
            "    entry: self.on_enter_parent;",
            "    state ChildA { entry: self.on_enter_child_a; }",
            "    state ChildB {}",
            "}"
        );
        let ast = parse_dsl(dsl).expect("DSL parsing failed ");
        let mut builder = TmpStateTreeBuilder::new();
        builder.build_from_ast(&ast).expect("Builder failed ");
        let machine_name_ident = &ast.name;
        let context_type_ast = &ast.context_type;
        let ids_info = crate::code_generator::generate_state_id_logic(&builder, machine_name_ident);
        let states_array_tokens =
            crate::code_generator::generate_states_array(&builder, &ids_info, context_type_ast)
                .expect("generate_states_array failed ");
        let expected_states_array_str = quote! {
            const STATES: &[StateNode<HierarchicalMachineStateId, AppContext>] = &[
                StateNode {
                    id: HierarchicalMachineStateId::Parent,
                    parent: None,
                    initial_child: Some(HierarchicalMachineStateId::ParentChildA),
                    entry_action: Some(self.on_enter_parent as ActionFn<AppContext>),
                    exit_action: None,
                    is_parallel: false, // Added
                },
                StateNode {
                    id: HierarchicalMachineStateId::ParentChildA,
                    parent: Some(HierarchicalMachineStateId::Parent),
                    initial_child: None,
                    entry_action: Some(self.on_enter_child_a as ActionFn<AppContext>),
                    exit_action: None,
                    is_parallel: false, // Added
                },
                StateNode {
                    id: HierarchicalMachineStateId::ParentChildB,
                    parent: Some(HierarchicalMachineStateId::Parent),
                    initial_child: None,
                    entry_action: None,
                    exit_action: None,
                    is_parallel: false, // Added
                }
            ];
        }
        .to_string();
        let normalize = |s: String| s.split_whitespace().collect::<Vec<&str>>().join(" ");
        assert_eq!(
            normalize(states_array_tokens.to_string()),
            normalize(expected_states_array_str)
        );
    }

    // Test for TRANSITIONS Array Generation (re-adding with updated DSL)
    #[test]
    fn generate_transitions_array_simple() {
        // ... (DSL string with event: Ev,) ...
        let dsl = concat!(
            "name: TestMachine, ",
            "context: Ctx, ",
            "event: Ev, ",
            "initial: S1, ",
            "state S1 { ",
            "    on E1 => S2; ",
            "    on E2 [guard g1] => S1; ",
            "} ",
            "state S2 { ",
            "    on E3 => S1 [action a1]; ",
            "}"
        );
        let ast = parse_dsl(dsl).expect("DSL parsing failed "); // Added space
        let mut builder = TmpStateTreeBuilder::new();
        builder.build_from_ast(&ast).expect("Builder failed "); // Added space
        let machine_name_ident = &ast.name;
        let context_type_ast = &ast.context_type;
        let event_type_ast = &ast.event_type;
        let ids_info = generate_state_id_logic(&builder, machine_name_ident);
        let transitions_array_tokens = crate::code_generator::generate_transitions_array(
            &builder,
            &ids_info,
            event_type_ast,
            context_type_ast,
        )
        .expect("generate_transitions_array failed "); // Added space
        let expected_str = quote! {
            const TRANSITIONS: &[Transition<TestMachineStateId, Ev, Ctx>] = &[
                Transition {
                    from_state: TestMachineStateId::S1,
                    event: Ev::E1,
                    to_state: TestMachineStateId::S2,
                    action: None,
                    guard: None,
                },
                Transition {
                    from_state: TestMachineStateId::S1,
                    event: Ev::E2,
                    to_state: TestMachineStateId::S1,
                    action: None,
                    guard: Some(g1 as GuardFn<Ctx, Ev>),
                },
                Transition {
                    from_state: TestMachineStateId::S2,
                    event: Ev::E3,
                    to_state: TestMachineStateId::S1,
                    action: Some(a1 as ActionFn<Ctx>),
                    guard: None,
                }
            ];
        }
        .to_string();
        let normalize = |s: String| s.split_whitespace().collect::<Vec<&str>>().join(" ");
        assert_eq!(
            normalize(transitions_array_tokens.to_string()),
            normalize(expected_str)
        );
    }

    #[test]
    fn generate_transitions_array_hierarchical() {
        // ... (DSL string with event: RootEv,) ...
        let dsl = concat!(
            "name: TestHierarchicalMachine, ",
            "context: RootCtx, ",
            "event: RootEv, ",
            "initial: P1, ",
            "state P1 { ",
            "    initial: C1; ",
            "    on E_P1_TO_C2 => C2; ",
            "    state C1 { ",
            "        initial: GC1; ",
            "        on E_C1_TO_GC2 => GC2; ",
            "        state GC1 { ",
            "            on E_GC1_TO_P2 => P2; ",
            "        } ",
            "        state GC2 {} ",
            "    } ",
            "    state C2 { ",
            "        on E_C2_TO_GC1 => P1::C1::GC1; ",
            "    } ",
            "} ",
            "state P2 {}"
        );
        let ast = parse_dsl(dsl).expect("DSL parsing failed "); // Added space
        let mut builder = TmpStateTreeBuilder::new();
        builder.build_from_ast(&ast).expect("Builder failed "); // Added space
        let machine_name_ident = &ast.name;
        let context_type_ast = &ast.context_type;
        let event_type_ast = &ast.event_type;
        let ids_info = generate_state_id_logic(&builder, machine_name_ident);

        let transitions_array_tokens = crate::code_generator::generate_transitions_array(
            &builder,
            &ids_info,
            event_type_ast,
            context_type_ast,
        )
        .expect("generate_transitions_array failed "); // Added space

        let expected_str = quote! {
            const TRANSITIONS: &[Transition<TestHierarchicalMachineStateId, RootEv, RootCtx>] = &[
                Transition {
                    from_state: TestHierarchicalMachineStateId::P1,
                    event: RootEv::E_P1_TO_C2,
                    to_state: TestHierarchicalMachineStateId::P1C2,
                    action: None,
                    guard: None,
                },
                Transition {
                    from_state: TestHierarchicalMachineStateId::P1C1,
                    event: RootEv::E_C1_TO_GC2,
                    to_state: TestHierarchicalMachineStateId::P1C1GC2, // Corrected Gc2 to GC2
                    action: None,
                    guard: None,
                },
                Transition {
                    from_state: TestHierarchicalMachineStateId::P1C1GC1,
                    event: RootEv::E_GC1_TO_P2,
                    to_state: TestHierarchicalMachineStateId::P2,
                    action: None,
                    guard: None,
                },
                Transition {
                    from_state: TestHierarchicalMachineStateId::P1C2,
                    event: RootEv::E_C2_TO_GC1,
                    to_state: TestHierarchicalMachineStateId::P1C1GC1,
                    action: None,
                    guard: None,
                }
            ];
        }
        .to_string();

        let normalize = |s: String| s.split_whitespace().collect::<Vec<&str>>().join(" ");
        assert_eq!(
            normalize(transitions_array_tokens.to_string()),
            normalize(expected_str)
        );
    }

    #[test]
    fn determine_initial_leaf_state_simple() {
        // ... (DSL string with event: Ev,) ...
        let dsl = concat!(
            "name: TestMachine, ",
            "context: Ctx, ",
            "event: Ev, ",
            "initial: S1, ",
            "state S1 {}",
            "state S2 {}"
        );
        let ast = parse_dsl(dsl).expect("DSL parsing failed "); // Added space
        let mut builder = TmpStateTreeBuilder::new();
        builder.build_from_ast(&ast).expect("Builder failed "); // Added space
        let ids_info = generate_state_id_logic(&builder, &ast.name);

        let initial_leaf_id_ts =
            crate::code_generator::determine_initial_leaf_state_id(&builder, &ids_info, &ast)
                .expect("determine_initial_leaf_state_id failed "); // Added space

        let expected_ts_str = quote! { TestMachineStateId::S1 }.to_string();
        assert_eq!(initial_leaf_id_ts.to_string(), expected_ts_str);
    }

    #[test]
    fn determine_initial_leaf_state_nested() {
        // ... (DSL string with event: Ev,) ...
        let dsl = concat!(
            "name: TestNested, ",
            "context: Ctx, ",
            "event: Ev, ",
            "initial: P1, ",
            "state P1 { ",
            "    initial: C1; ",
            "    state C1 { ",
            "        initial: GC1; ",
            "        state GC1 {} ",
            "        state GC2 {} ",
            "    } ",
            "    state C2 {} ",
            "} ",
            "state P2 {}"
        );
        let ast = parse_dsl(dsl).expect("DSL parsing failed "); // Added space
        let mut builder = TmpStateTreeBuilder::new();
        builder.build_from_ast(&ast).expect("Builder failed "); // Added space
        let ids_info = generate_state_id_logic(&builder, &ast.name);

        let initial_leaf_id_ts =
            crate::code_generator::determine_initial_leaf_state_id(&builder, &ids_info, &ast)
                .expect("determine_initial_leaf_state_id failed "); // Added space

        // Expected leaf: P1 -> C1 -> GC1. StateId: TestNestedStateId::P1C1GC1
        let expected_ts_str = quote! { TestNestedStateId::P1C1GC1 }.to_string();
        assert_eq!(initial_leaf_id_ts.to_string(), expected_ts_str);
    }

    #[test]
    fn determine_initial_leaf_state_target_not_top_level_error() {
        // ... (DSL string with event: Ev,) ...
        let dsl = concat!(
            "name: TestMachine, ",
            "context: Ctx, ",
            "event: Ev, ",
            "initial: S1_S1_Child, ", // Target a nested state
            "state S1 { initial: S1_Child; state S1_Child {} }"  // S1 is valid, S1_S1_Child exists
        );
        let ast = parse_dsl(dsl).expect("DSL parsing failed "); // Added space
        let mut builder = TmpStateTreeBuilder::new();
        builder
            .build_from_ast(&ast)
            .expect("Builder should succeed with this valid AST "); // Builder should pass
        let ids_info = generate_state_id_logic(&builder, &ast.name);

        let result =
            crate::code_generator::determine_initial_leaf_state_id(&builder, &ids_info, &ast);
        assert!(
            result.is_err(),
            "Expected error for initial target not being top-level "
        ); // Added space
        if let Err(e) = result {
            assert!(
                e.to_string().contains(
                    "Declared top-level initial state 'S1_S1_Child' is not a top-level state."
                ),
                "Unexpected error message: {e}"
            ); // Ensured double quotes
        }
    }

    #[test]
    fn determine_initial_leaf_state_non_existent_error() {
        // ... (DSL string with event: Ev,) ...
        let dsl = concat!(
            "name: TestMachine, ",
            "context: Ctx, ",
            "event: Ev, ",
            "initial: NonExistentState, ",
            "state S1 {}"
        );
        let ast = parse_dsl(dsl).expect("DSL parsing failed "); // Added space
        let mut builder = TmpStateTreeBuilder::new();
        builder
            .build_from_ast(&ast)
            .expect("Builder should succeed initially "); // Added space
        let ids_info = generate_state_id_logic(&builder, &ast.name);

        let result =
            crate::code_generator::determine_initial_leaf_state_id(&builder, &ids_info, &ast);
        assert!(
            result.is_err(),
            "Expected error for non-existent initial target "
        ); // Added space
        if let Err(e) = result {
            assert!(e
                .to_string()
                .contains("Declared top-level initial state 'NonExistentState' not found."));
            // Ensured double quotes and closing quote
        }
    }
    // Note: The test case `parse_state_chart_input_extra_tokens_after_states` also uses an `expect` that needs a space.
    // And `parse_transition_with_action_only_implicit_keyword`
    // And `parse_transition_action_ast_leading_dot_error`
    // And `transition_target_unknown_path_errors`

    #[test]
    #[allow(clippy::too_many_lines)] // Allow long test for comprehensive showcase validation
    fn parse_and_build_hierarchical_showcase_example() {
        let dsl = r"
            name: Agent,
            context: AgentCtx,
            event: AgentEvent, // Assuming AgentEvent is a parsable Path
            initial: Operational,

            state Operational {
                initial: Idle;
                on ReportError => Errored [action self.log_error]; // Simplified action for testing

                state Idle {
                    on Activate [guard self.can_start] => Active [action self.start_up];
                    // after 5s => Active [action self.start_up]; // `after` not yet parsed/handled
                }

                state Active {
                    on Deactivate => Idle [action self.shut_down];
                    on Activate [guard self.can_start] => Active [action self.start_up];
                }
            }

            state Errored {
                on Deactivate => Operational;
            }
        ";

        let ast = parse_dsl(dsl).expect("DSL parsing for showcase example failed");
        let mut builder = TmpStateTreeBuilder::new();
        let build_result = builder.build_from_ast(&ast);
        assert!(
            build_result.is_ok(),
            "Builder failed for showcase example: {:?}",
            build_result.err()
        );

        // Basic checks on the builder's state
        assert_eq!(
            builder.all_states.len(),
            4,
            "Expected 4 states: Operational, Idle, Active, Errored"
        );
        assert_eq!(builder.state_full_path_to_idx_map.len(), 4);

        // Check Operational state
        let operational_idx = *builder
            .state_full_path_to_idx_map
            .get("Operational")
            .expect("Operational state not found in map");
        let operational_state = &builder.all_states[operational_idx];
        assert_eq!(operational_state.local_name.to_string(), "Operational");
        assert_eq!(operational_state.full_path_name, "Operational");
        assert!(operational_state.parent_full_path_name.is_none());
        assert_eq!(operational_state.depth, 0);
        assert_eq!(
            operational_state.children_indices.len(),
            2,
            "Operational should have Idle and Active as children"
        );

        let idle_idx_direct = *builder
            .state_full_path_to_idx_map
            .get("Operational_Idle")
            .expect("Operational_Idle not found");
        assert_eq!(
            operational_state.initial_child_idx,
            Some(idle_idx_direct),
            "Operational initial child should be Idle"
        );
        assert_eq!(operational_state.transitions.len(), 1); // on ReportError
        assert_eq!(
            operational_state.transitions[0].event_name.to_string(),
            "ReportError"
        );
        let target_errored_idx = *builder.state_full_path_to_idx_map.get("Errored").unwrap();
        assert_eq!(
            operational_state.transitions[0].target_state_idx,
            Some(target_errored_idx)
        );

        // Check Idle state (child of Operational)
        let idle_idx = *builder
            .state_full_path_to_idx_map
            .get("Operational_Idle")
            .expect("Idle state not found in map");
        let idle_state = &builder.all_states[idle_idx];
        assert_eq!(idle_state.local_name.to_string(), "Idle");
        assert_eq!(idle_state.full_path_name, "Operational_Idle");
        assert_eq!(
            idle_state.parent_full_path_name,
            Some("Operational".to_string())
        );
        assert_eq!(idle_state.depth, 1);
        assert!(idle_state.children_indices.is_empty());
        assert!(idle_state.initial_child_idx.is_none());
        assert_eq!(idle_state.transitions.len(), 1); // on Activate
        assert_eq!(idle_state.transitions[0].event_name.to_string(), "Activate");
        let active_idx_direct = *builder
            .state_full_path_to_idx_map
            .get("Operational_Active")
            .unwrap();
        assert_eq!(
            idle_state.transitions[0].target_state_idx,
            Some(active_idx_direct)
        );

        // Check Active state (child of Operational)
        let active_idx = *builder
            .state_full_path_to_idx_map
            .get("Operational_Active")
            .expect("Active state not found in map");
        let active_state = &builder.all_states[active_idx];
        assert_eq!(active_state.local_name.to_string(), "Active");
        assert_eq!(active_state.full_path_name, "Operational_Active");
        assert_eq!(
            active_state.parent_full_path_name,
            Some("Operational".to_string())
        );
        assert_eq!(active_state.depth, 1);
        assert!(active_state.children_indices.is_empty());
        assert!(active_state.initial_child_idx.is_none());
        assert_eq!(active_state.transitions.len(), 2); // on Deactivate, on Activate (self)
        assert_eq!(
            active_state.transitions[0].event_name.to_string(),
            "Deactivate"
        );
        assert_eq!(
            active_state.transitions[0].target_state_idx,
            Some(idle_idx_direct)
        );
        assert_eq!(
            active_state.transitions[1].event_name.to_string(),
            "Activate"
        );
        assert_eq!(
            active_state.transitions[1].target_state_idx,
            Some(active_idx_direct)
        ); // Self-transition

        // Check Errored state
        let errored_idx = *builder
            .state_full_path_to_idx_map
            .get("Errored")
            .expect("Errored state not found in map");
        let errored_state = &builder.all_states[errored_idx];
        assert_eq!(errored_state.local_name.to_string(), "Errored");
        assert_eq!(errored_state.full_path_name, "Errored");
        assert!(errored_state.parent_full_path_name.is_none());
        assert_eq!(errored_state.depth, 0);
        assert!(errored_state.children_indices.is_empty());
        assert!(errored_state.initial_child_idx.is_none());
        assert_eq!(errored_state.transitions.len(), 1); // on Deactivate
        assert_eq!(
            errored_state.transitions[0].event_name.to_string(),
            "Deactivate"
        );
        assert_eq!(
            errored_state.transitions[0].target_state_idx,
            Some(operational_idx)
        );

        // Check code generation parts (simple checks, not full output validation)
        let ids_info = generate_state_id_logic(&builder, &ast.name);
        assert_eq!(ids_info.state_id_enum_name.to_string(), "AgentStateId");
        assert_eq!(ids_info.full_path_to_variant_ident.len(), 4);
        assert_eq!(
            ids_info
                .full_path_to_variant_ident
                .get("Operational")
                .unwrap()
                .to_string(),
            "Operational"
        );
        assert_eq!(
            ids_info
                .full_path_to_variant_ident
                .get("Operational_Idle")
                .unwrap()
                .to_string(),
            "OperationalIdle"
        );
        assert_eq!(
            ids_info
                .full_path_to_variant_ident
                .get("Operational_Active")
                .unwrap()
                .to_string(),
            "OperationalActive"
        );
        assert_eq!(
            ids_info
                .full_path_to_variant_ident
                .get("Errored")
                .unwrap()
                .to_string(),
            "Errored"
        );

        let initial_leaf_token_stream =
            crate::code_generator::determine_initial_leaf_state_id(&builder, &ids_info, &ast)
                .expect("Determine initial leaf state failed for showcase");
        // Operational -> initial: Idle. So leaf is OperationalIdle
        assert_eq!(
            initial_leaf_token_stream.to_string(),
            "AgentStateId :: OperationalIdle"
        );

        // Test generation of STATES array (basic check)
        let states_array_result =
            crate::code_generator::generate_states_array(&builder, &ids_info, &ast.context_type);
        assert!(
            states_array_result.is_ok(),
            "generate_states_array failed: {:?}",
            states_array_result.err()
        );
        let states_array_str = states_array_result.unwrap().to_string();
        assert!(states_array_str.contains("id : AgentStateId :: OperationalIdle"));
        assert!(states_array_str.contains("parent : Some (AgentStateId :: Operational)")); // For Idle
        assert!(states_array_str.contains("initial_child : Some (AgentStateId :: OperationalIdle)")); // For Operational

        // Test generation of TRANSITIONS array (basic check)
        let transitions_array_result = crate::code_generator::generate_transitions_array(
            &builder,
            &ids_info,
            &ast.event_type,
            &ast.context_type,
        );
        assert!(
            transitions_array_result.is_ok(),
            "generate_transitions_array failed: {:?}",
            transitions_array_result.err()
        );
        let transitions_array_str = transitions_array_result.unwrap().to_string();
        assert!(transitions_array_str.contains("from_state : AgentStateId :: OperationalIdle"));
        assert!(transitions_array_str.contains("to_state : AgentStateId :: OperationalActive"));
        assert!(transitions_array_str.contains("from_state : AgentStateId :: Operational"));
        assert!(transitions_array_str.contains("to_state : AgentStateId :: Errored"));
    }

    #[test]
    fn parse_state_with_parallel_attribute() {
        let input_dsl = r"
            state MyState [parallel] {
                initial: A;
                state A {}
            }
        "; // Removed #
        let result: Result<StateDeclarationAst> = syn::parse_str(input_dsl);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
        let state_decl = result.unwrap();
        assert_eq!(state_decl.name.to_string(), "MyState");
        assert!(state_decl.attributes.is_some(), "Attributes should be Some");
        let attrs_input = state_decl.attributes.unwrap();
        assert_eq!(attrs_input.attributes.len(), 1);
        let parsed_attr = attrs_input.attributes.first().unwrap(); // Removed second unwrap
        match parsed_attr {
            StateAttributeAst::Parallel(_) => { /* Correct */ }
        }
        assert!(state_decl.default_child_declaration.is_some());
    }

    #[test]
    fn parse_state_with_parallel_attribute_trailing_comma() {
        let input_dsl = r"
            state MyState [parallel,] {
                initial: A;
                state A {}
            }
        ";
        let result: Result<StateDeclarationAst> = syn::parse_str(input_dsl);
        assert!(
            result.is_ok(),
            "Failed to parse with trailing comma: {:?}",
            result.err()
        );
        let state_decl = result.unwrap();
        assert!(state_decl.attributes.is_some());
        let attributes_input_ast = state_decl.attributes.unwrap(); // Extended lifetime
        assert_eq!(attributes_input_ast.attributes.len(), 1);
        let parsed_attr = attributes_input_ast.attributes.first().unwrap(); // Corrected
        match parsed_attr {
            StateAttributeAst::Parallel(_) => { /* Correct */ }
        }
    }

    #[test]
    fn parse_state_without_attributes() {
        let input_dsl = r"
            state MyState {
                initial: A;
                state A {}
            }
        "; // Removed #
        let result: Result<StateDeclarationAst> = syn::parse_str(input_dsl);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
        let state_decl = result.unwrap();
        assert_eq!(state_decl.name.to_string(), "MyState");
        assert!(state_decl.attributes.is_none(), "Attributes should be None");
    }

    #[test]
    fn parse_state_with_empty_attributes_should_error() {
        let input_dsl = r"
            state MyState [] { // Empty brackets
                initial: A;
                state A {}
            }
        ";
        let result: Result<StateDeclarationAst> = syn::parse_str(input_dsl);
        assert!(
            result.is_err(),
            "Parsing empty attribute brackets should now error due to StateAttributesInputAst validation"
        );
        if let Err(e) = result {
            // Check for the new error message
            assert!(
                e.to_string()
                    .contains("State attribute list cannot be empty if brackets are present"),
                "Error message mismatch for empty attributes: {e}"
            );
        }
    }

    #[test]
    fn parse_state_with_unknown_attribute_should_error() {
        let input_dsl = r"
            state MyState [foo] { // Unknown attribute
                initial: A;
                state A {}
            }
        ";
        let result: Result<StateDeclarationAst> = syn::parse_str(input_dsl);
        assert!(result.is_err(), "Parsing unknown attribute should error");
        if let Err(e) = result {
            assert!(
                e.to_string()
                    .contains("Expected 'parallel' attribute within state attribute brackets"),
                "Error message mismatch: {e}" // Inlined e
            );
        }
    }
}
