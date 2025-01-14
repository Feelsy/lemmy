use super::*;
use crate::api::user::Register;
use crate::api::{Oper, Perform};
use crate::settings::Settings;
use diesel::PgConnection;
use log::info;
use std::str::FromStr;

#[derive(Serialize, Deserialize)]
pub struct ListCategories {}

#[derive(Serialize, Deserialize)]
pub struct ListCategoriesResponse {
  categories: Vec<Category>,
}

#[derive(Serialize, Deserialize)]
pub struct Search {
  q: String,
  type_: String,
  community_id: Option<i32>,
  sort: String,
  page: Option<i64>,
  limit: Option<i64>,
  auth: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct SearchResponse {
  type_: String,
  comments: Vec<CommentView>,
  posts: Vec<PostView>,
  communities: Vec<CommunityView>,
  users: Vec<UserView>,
}

#[derive(Serialize, Deserialize)]
pub struct GetModlog {
  mod_user_id: Option<i32>,
  community_id: Option<i32>,
  page: Option<i64>,
  limit: Option<i64>,
}

#[derive(Serialize, Deserialize)]
pub struct GetModlogResponse {
  removed_posts: Vec<ModRemovePostView>,
  locked_posts: Vec<ModLockPostView>,
  stickied_posts: Vec<ModStickyPostView>,
  removed_comments: Vec<ModRemoveCommentView>,
  removed_communities: Vec<ModRemoveCommunityView>,
  banned_from_community: Vec<ModBanFromCommunityView>,
  banned: Vec<ModBanView>,
  added_to_community: Vec<ModAddCommunityView>,
  added: Vec<ModAddView>,
}

#[derive(Serialize, Deserialize)]
pub struct CreateSite {
  pub name: String,
  pub description: Option<String>,
  pub enable_downvotes: bool,
  pub open_registration: bool,
  pub enable_nsfw: bool,
  pub auth: String,
}

#[derive(Serialize, Deserialize)]
pub struct EditSite {
  name: String,
  description: Option<String>,
  enable_downvotes: bool,
  open_registration: bool,
  enable_nsfw: bool,
  auth: String,
}

#[derive(Serialize, Deserialize)]
pub struct GetSite {}

#[derive(Serialize, Deserialize)]
pub struct SiteResponse {
  site: SiteView,
}

#[derive(Serialize, Deserialize)]
pub struct GetSiteResponse {
  site: Option<SiteView>,
  admins: Vec<UserView>,
  banned: Vec<UserView>,
  pub online: usize,
}

#[derive(Serialize, Deserialize)]
pub struct TransferSite {
  user_id: i32,
  auth: String,
}

impl Perform<ListCategoriesResponse> for Oper<ListCategories> {
  fn perform(&self, conn: &PgConnection) -> Result<ListCategoriesResponse, Error> {
    let _data: &ListCategories = &self.data;

    let categories: Vec<Category> = Category::list_all(&conn)?;

    // Return the jwt
    Ok(ListCategoriesResponse { categories })
  }
}

impl Perform<GetModlogResponse> for Oper<GetModlog> {
  fn perform(&self, conn: &PgConnection) -> Result<GetModlogResponse, Error> {
    let data: &GetModlog = &self.data;

    let removed_posts = ModRemovePostView::list(
      &conn,
      data.community_id,
      data.mod_user_id,
      data.page,
      data.limit,
    )?;
    let locked_posts = ModLockPostView::list(
      &conn,
      data.community_id,
      data.mod_user_id,
      data.page,
      data.limit,
    )?;
    let stickied_posts = ModStickyPostView::list(
      &conn,
      data.community_id,
      data.mod_user_id,
      data.page,
      data.limit,
    )?;
    let removed_comments = ModRemoveCommentView::list(
      &conn,
      data.community_id,
      data.mod_user_id,
      data.page,
      data.limit,
    )?;
    let banned_from_community = ModBanFromCommunityView::list(
      &conn,
      data.community_id,
      data.mod_user_id,
      data.page,
      data.limit,
    )?;
    let added_to_community = ModAddCommunityView::list(
      &conn,
      data.community_id,
      data.mod_user_id,
      data.page,
      data.limit,
    )?;

    // These arrays are only for the full modlog, when a community isn't given
    let (removed_communities, banned, added) = if data.community_id.is_none() {
      (
        ModRemoveCommunityView::list(&conn, data.mod_user_id, data.page, data.limit)?,
        ModBanView::list(&conn, data.mod_user_id, data.page, data.limit)?,
        ModAddView::list(&conn, data.mod_user_id, data.page, data.limit)?,
      )
    } else {
      (Vec::new(), Vec::new(), Vec::new())
    };

    // Return the jwt
    Ok(GetModlogResponse {
      removed_posts,
      locked_posts,
      stickied_posts,
      removed_comments,
      removed_communities,
      banned_from_community,
      banned,
      added_to_community,
      added,
    })
  }
}

impl Perform<SiteResponse> for Oper<CreateSite> {
  fn perform(&self, conn: &PgConnection) -> Result<SiteResponse, Error> {
    let data: &CreateSite = &self.data;

    let claims = match Claims::decode(&data.auth) {
      Ok(claims) => claims.claims,
      Err(_e) => return Err(APIError::err("not_logged_in").into()),
    };

    if let Err(slurs) = slur_check(&data.name) {
      return Err(APIError::err(&slurs_vec_to_str(slurs)).into());
    }

    if let Some(description) = &data.description {
      if let Err(slurs) = slur_check(description) {
        return Err(APIError::err(&slurs_vec_to_str(slurs)).into());
      }
    }

    let user_id = claims.id;

    // Make sure user is an admin
    if !UserView::read(&conn, user_id)?.admin {
      return Err(APIError::err("not_an_admin").into());
    }

    let site_form = SiteForm {
      name: data.name.to_owned(),
      description: data.description.to_owned(),
      creator_id: user_id,
      enable_downvotes: data.enable_downvotes,
      open_registration: data.open_registration,
      enable_nsfw: data.enable_nsfw,
      updated: None,
    };

    match Site::create(&conn, &site_form) {
      Ok(site) => site,
      Err(_e) => return Err(APIError::err("site_already_exists").into()),
    };

    let site_view = SiteView::read(&conn)?;

    Ok(SiteResponse { site: site_view })
  }
}

impl Perform<SiteResponse> for Oper<EditSite> {
  fn perform(&self, conn: &PgConnection) -> Result<SiteResponse, Error> {
    let data: &EditSite = &self.data;

    let claims = match Claims::decode(&data.auth) {
      Ok(claims) => claims.claims,
      Err(_e) => return Err(APIError::err("not_logged_in").into()),
    };

    if let Err(slurs) = slur_check(&data.name) {
      return Err(APIError::err(&slurs_vec_to_str(slurs)).into());
    }

    if let Some(description) = &data.description {
      if let Err(slurs) = slur_check(description) {
        return Err(APIError::err(&slurs_vec_to_str(slurs)).into());
      }
    }

    let user_id = claims.id;

    // Make sure user is an admin
    if !UserView::read(&conn, user_id)?.admin {
      return Err(APIError::err("not_an_admin").into());
    }

    let found_site = Site::read(&conn, 1)?;

    let site_form = SiteForm {
      name: data.name.to_owned(),
      description: data.description.to_owned(),
      creator_id: found_site.creator_id,
      updated: Some(naive_now()),
      enable_downvotes: data.enable_downvotes,
      open_registration: data.open_registration,
      enable_nsfw: data.enable_nsfw,
    };

    match Site::update(&conn, 1, &site_form) {
      Ok(site) => site,
      Err(_e) => return Err(APIError::err("couldnt_update_site").into()),
    };

    let site_view = SiteView::read(&conn)?;

    Ok(SiteResponse { site: site_view })
  }
}

impl Perform<GetSiteResponse> for Oper<GetSite> {
  fn perform(&self, conn: &PgConnection) -> Result<GetSiteResponse, Error> {
    let _data: &GetSite = &self.data;

    let site = Site::read(&conn, 1);
    let site_view = if site.is_ok() {
      Some(SiteView::read(&conn)?)
    } else if let Some(setup) = Settings::get().setup.as_ref() {
      let register = Register {
        username: setup.admin_username.to_owned(),
        email: setup.admin_email.to_owned(),
        password: setup.admin_password.to_owned(),
        password_verify: setup.admin_password.to_owned(),
        admin: true,
        show_nsfw: true,
      };
      let login_response = Oper::new(register).perform(&conn)?;
      info!("Admin {} created", setup.admin_username);

      let create_site = CreateSite {
        name: setup.site_name.to_owned(),
        description: None,
        enable_downvotes: false,
        open_registration: false,
        enable_nsfw: false,
        auth: login_response.jwt,
      };
      Oper::new(create_site).perform(&conn)?;
      info!("Site {} created", setup.site_name);
      Some(SiteView::read(&conn)?)
    } else {
      None
    };

    let mut admins = UserView::admins(&conn)?;
    if site_view.is_some() {
      let site_creator_id = site_view.to_owned().unwrap().creator_id;
      let creator_index = admins.iter().position(|r| r.id == site_creator_id).unwrap();
      let creator_user = admins.remove(creator_index);
      admins.insert(0, creator_user);
    }

    let banned = UserView::banned(&conn)?;

    Ok(GetSiteResponse {
      site: site_view,
      admins,
      banned,
      online: 0,
    })
  }
}

impl Perform<SearchResponse> for Oper<Search> {
  fn perform(&self, conn: &PgConnection) -> Result<SearchResponse, Error> {
    let data: &Search = &self.data;

    let user_id: Option<i32> = match &data.auth {
      Some(auth) => match Claims::decode(&auth) {
        Ok(claims) => {
          let user_id = claims.claims.id;
          Some(user_id)
        }
        Err(_e) => None,
      },
      None => None,
    };

    let sort = SortType::from_str(&data.sort)?;
    let type_ = SearchType::from_str(&data.type_)?;

    let mut posts = Vec::new();
    let mut comments = Vec::new();
    let mut communities = Vec::new();
    let mut users = Vec::new();

    // TODO no clean / non-nsfw searching rn

    match type_ {
      SearchType::Posts => {
        posts = PostQueryBuilder::create(&conn)
          .sort(&sort)
          .show_nsfw(true)
          .for_community_id(data.community_id)
          .search_term(data.q.to_owned())
          .my_user_id(user_id)
          .page(data.page)
          .limit(data.limit)
          .list()?;
      }
      SearchType::Comments => {
        comments = CommentQueryBuilder::create(&conn)
          .sort(&sort)
          .search_term(data.q.to_owned())
          .my_user_id(user_id)
          .page(data.page)
          .limit(data.limit)
          .list()?;
      }
      SearchType::Communities => {
        communities = CommunityQueryBuilder::create(&conn)
          .sort(&sort)
          .search_term(data.q.to_owned())
          .page(data.page)
          .limit(data.limit)
          .list()?;
      }
      SearchType::Users => {
        users = UserQueryBuilder::create(&conn)
          .sort(&sort)
          .search_term(data.q.to_owned())
          .page(data.page)
          .limit(data.limit)
          .list()?;
      }
      SearchType::All => {
        posts = PostQueryBuilder::create(&conn)
          .sort(&sort)
          .show_nsfw(true)
          .for_community_id(data.community_id)
          .search_term(data.q.to_owned())
          .my_user_id(user_id)
          .page(data.page)
          .limit(data.limit)
          .list()?;

        comments = CommentQueryBuilder::create(&conn)
          .sort(&sort)
          .search_term(data.q.to_owned())
          .my_user_id(user_id)
          .page(data.page)
          .limit(data.limit)
          .list()?;

        communities = CommunityQueryBuilder::create(&conn)
          .sort(&sort)
          .search_term(data.q.to_owned())
          .page(data.page)
          .limit(data.limit)
          .list()?;

        users = UserQueryBuilder::create(&conn)
          .sort(&sort)
          .search_term(data.q.to_owned())
          .page(data.page)
          .limit(data.limit)
          .list()?;
      }
      SearchType::Url => {
        posts = PostQueryBuilder::create(&conn)
          .sort(&sort)
          .show_nsfw(true)
          .for_community_id(data.community_id)
          .url_search(data.q.to_owned())
          .page(data.page)
          .limit(data.limit)
          .list()?;
      }
    };

    // Return the jwt
    Ok(SearchResponse {
      type_: data.type_.to_owned(),
      comments,
      posts,
      communities,
      users,
    })
  }
}

impl Perform<GetSiteResponse> for Oper<TransferSite> {
  fn perform(&self, conn: &PgConnection) -> Result<GetSiteResponse, Error> {
    let data: &TransferSite = &self.data;

    let claims = match Claims::decode(&data.auth) {
      Ok(claims) => claims.claims,
      Err(_e) => return Err(APIError::err("not_logged_in").into()),
    };

    let user_id = claims.id;

    let read_site = Site::read(&conn, 1)?;

    // Make sure user is the creator
    if read_site.creator_id != user_id {
      return Err(APIError::err("not_an_admin").into());
    }

    let site_form = SiteForm {
      name: read_site.name,
      description: read_site.description,
      creator_id: data.user_id,
      updated: Some(naive_now()),
      enable_downvotes: read_site.enable_downvotes,
      open_registration: read_site.open_registration,
      enable_nsfw: read_site.enable_nsfw,
    };

    match Site::update(&conn, 1, &site_form) {
      Ok(site) => site,
      Err(_e) => return Err(APIError::err("couldnt_update_site").into()),
    };

    // Mod tables
    let form = ModAddForm {
      mod_user_id: user_id,
      other_user_id: data.user_id,
      removed: Some(false),
    };

    ModAdd::create(&conn, &form)?;

    let site_view = SiteView::read(&conn)?;

    let mut admins = UserView::admins(&conn)?;
    let creator_index = admins
      .iter()
      .position(|r| r.id == site_view.creator_id)
      .unwrap();
    let creator_user = admins.remove(creator_index);
    admins.insert(0, creator_user);

    let banned = UserView::banned(&conn)?;

    Ok(GetSiteResponse {
      site: Some(site_view),
      admins,
      banned,
      online: 0,
    })
  }
}
